use crate::SqliteReadOnlyConnection;
use std::ffi::CStr;
use std::ffi::{c_char, c_int, c_void};
use std::marker::PhantomData;

#[cfg(target_os = "macos")]
#[link(name = "sqlite3")]
unsafe extern "C" {
    #[link_name = "sqlite3_prepare_v2"]
    fn statement_prepare(
        database: *mut c_void,
        text: *const c_char,
        length: c_int,
        statement: *mut *mut c_void,
        tail: *mut *const c_char,
    ) -> c_int;
    #[link_name = "sqlite3_step"]
    fn statement_step(statement: *mut c_void) -> c_int;
    #[link_name = "sqlite3_column_type"]
    fn statement_column_type(statement: *mut c_void, column: c_int) -> c_int;
    #[link_name = "sqlite3_column_text"]
    fn statement_column_text(statement: *mut c_void, column: c_int) -> *const c_char;
    #[link_name = "sqlite3_column_int64"]
    fn statement_column_integer(statement: *mut c_void, column: c_int) -> i64;
    #[link_name = "sqlite3_finalize"]
    fn statement_finalize(statement: *mut c_void) -> c_int;
}

pub struct SqliteReadOnlyStatement<'connection> {
    handle: *mut c_void,
    connection: PhantomData<&'connection SqliteReadOnlyConnection>,
}

#[cfg(target_os = "macos")]
impl<'connection> SqliteReadOnlyStatement<'connection> {
    const SUCCESS: c_int = 0;
    const ROW: c_int = 100;
    const NULL_TYPE: c_int = 5;
    const MAXIMUM_ROWS: usize = 5_000;

    pub fn prepare(
        query: &'static str,
        connection: &'connection SqliteReadOnlyConnection,
    ) -> Option<Self> {
        let mut handle: *mut c_void = std::ptr::null_mut();
        let status: c_int = Self::prepare_statement(connection.handle(), query, &mut handle);
        let statement: Self = Self {
            handle,
            connection: PhantomData,
        };
        statement.ready(status)
    }

    pub fn rows<T>(&mut self, read: impl Fn(&Self) -> Option<T>) -> Vec<T> {
        self.collect(read)
    }

    pub fn text(&self, column: c_int) -> Option<String> {
        let pointer: *const c_char = self.column_text(column);
        Self::owned_text(pointer)
    }

    pub fn integer(&self, column: c_int) -> Option<i64> {
        let present: bool = self.column_type(column) != Self::NULL_TYPE;
        present.then(|| -> i64 { self.column_integer(column) })
    }

    fn ready(self, status: c_int) -> Option<Self> {
        let prepared: bool = [status == Self::SUCCESS, !self.handle.is_null()]
            .into_iter()
            .all(std::convert::identity);
        prepared.then_some(self)
    }

    fn collect<T>(&mut self, read: impl Fn(&Self) -> Option<T>) -> Vec<T> {
        let mut rows: Vec<T> = Vec::new();
        while self.next_row(rows.len()) {
            rows.extend(read(self));
        }
        rows
    }

    fn next_row(&mut self, count: usize) -> bool {
        if count >= Self::MAXIMUM_ROWS {
            return false;
        }
        self.step() == Self::ROW
    }

    fn owned_text(pointer: *const c_char) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        unsafe { CStr::from_ptr(pointer) }
            .to_str()
            .ok()
            .map(str::to_owned)
    }

    fn prepare_statement(database: *mut c_void, query: &str, handle: &mut *mut c_void) -> c_int {
        let length: c_int = c_int::try_from(query.len()).unwrap_or_default();
        unsafe {
            statement_prepare(
                database,
                query.as_ptr().cast(),
                length,
                handle,
                std::ptr::null_mut(),
            )
        }
    }

    fn step(&mut self) -> c_int {
        unsafe { statement_step(self.handle) }
    }

    fn column_type(&self, column: c_int) -> c_int {
        unsafe { statement_column_type(self.handle, column) }
    }

    fn column_text(&self, column: c_int) -> *const c_char {
        unsafe { statement_column_text(self.handle, column) }
    }

    fn column_integer(&self, column: c_int) -> i64 {
        unsafe { statement_column_integer(self.handle, column) }
    }
}

#[cfg(target_os = "macos")]
impl Drop for SqliteReadOnlyStatement<'_> {
    fn drop(&mut self) -> () {
        unsafe { statement_finalize(self.handle) };
    }
}

#[cfg(not(target_os = "macos"))]
impl<'connection> SqliteReadOnlyStatement<'connection> {
    pub fn prepare(
        _query: &'static str,
        _connection: &'connection SqliteReadOnlyConnection,
    ) -> Option<Self> {
        None
    }

    pub fn rows<T>(&mut self, _read: impl Fn(&Self) -> Option<T>) -> Vec<T> {
        Vec::new()
    }

    pub fn text(&self, _column: c_int) -> Option<String> {
        None
    }

    pub fn integer(&self, _column: c_int) -> Option<i64> {
        None
    }
}

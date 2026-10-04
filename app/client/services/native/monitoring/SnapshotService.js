@Service
export default class SnapshotService {
    /** @type {number} */
    static _MAXIMUM_BYTES = 2_097_152;
    /** @type {number} */
    static _MAXIMUM_PAGES = 86;
    /** @type {number} */
    static _MINIMUM = 0;
    /** @type {string} */
    static _EMPTY_TEXT = '';
    /** @type {string} */
    static _BUSY_READ = 'A local Overview read is already in progress.';
    /** @type {import('/services/SnapshotReaderService.js').default | undefined} */
    @Needs
    snapshotReader;
    /** @type {Promise<import('/models/ObservatoryModel.ts').ObservatoryModel> | null} */
    #pending = null;
    /** @param {import('/models/NativeViewSelection.ts').NativeViewSelection} selection @returns {Promise<import('/models/ObservatoryModel.ts').ObservatoryModel>} */
    read(selection) {
        return this.#currentRead(selection);
    }
    /** @param {import('/models/NativeViewSelection.ts').NativeViewSelection} selection @returns {Promise<import('/models/ObservatoryModel.ts').ObservatoryModel>} */
    #currentRead(selection) {
        if (this.#pending !== null) {
            return Promise.reject(new Error(SnapshotService._BUSY_READ));
        }
        this.#pending = this.#transfer(selection).finally(/** @returns {void} */ () => this.#clearPending());
        return this.#pending;
    }
    /** @returns {void} */
    #clearPending() {
        this.#pending = null;
    }
    /** @param {import('/models/NativeViewSelection.ts').NativeViewSelection} selection @returns {Promise<import('/models/ObservatoryModel.ts').ObservatoryModel>} */
    async #transfer(selection) {
        const reader = this.#reader();
        const ticket = await reader.open(selection);
        this.#requireTicket(ticket);
        return this.#readAndRelease(ticket, reader);
    }
    /** @param {import('/models/SnapshotTransfer.ts').SnapshotTransfer} ticket @param {import('/services/SnapshotReaderService.js').default} reader @returns {Promise<import('/models/ObservatoryModel.ts').ObservatoryModel>} */
    #readAndRelease(ticket, reader) {
        return this.#readContent(ticket, reader).finally(/** @returns {Promise<boolean>} */ () => reader.release(ticket.id));
    }
    /** @param {import('/models/SnapshotTransfer.ts').SnapshotTransfer} ticket @param {import('/services/SnapshotReaderService.js').default} reader @returns {Promise<import('/models/ObservatoryModel.ts').ObservatoryModel>} */
    async #readContent(ticket, reader) {
        const content = await this.#pages(ticket, reader);
        return this.#decode(content, ticket);
    }
    /** @param {import('/models/SnapshotTransfer.ts').SnapshotTransfer} ticket @returns {void} */
    #requireTicket(ticket) {
        const checks = [ticket.error === SnapshotService._EMPTY_TEXT, Number.isSafeInteger(ticket.id), ticket.id > SnapshotService._MINIMUM,
            Number.isSafeInteger(ticket.pages), ticket.pages > SnapshotService._MINIMUM, ticket.pages <= SnapshotService._MAXIMUM_PAGES,
            Number.isSafeInteger(ticket.bytes), ticket.bytes > SnapshotService._MINIMUM, ticket.bytes <= SnapshotService._MAXIMUM_BYTES];
        this.#require(checks.every(Boolean), 'Invalid local snapshot transfer. ' + ticket.error);
    }
    /** @param {import('/models/SnapshotTransfer.ts').SnapshotTransfer} ticket @param {import('/services/SnapshotReaderService.js').default} reader @returns {Promise<string>} */
    async #pages(ticket, reader) {
        /** @type {string[]} */
        const parts = [];
        for (let index = 0; index < ticket.pages; index += 1) {
            parts.push(await this.#part(ticket.id, index, reader));
        }
        return parts.join(SnapshotService._EMPTY_TEXT);
    }
    /** @param {number} id @param {number} index @param {import('/services/SnapshotReaderService.js').default} reader @returns {Promise<string>} */
    async #part(id, index, reader) {
        const page = await reader.page(id, index);
        const checks = [page.error === SnapshotService._EMPTY_TEXT, page.id === id, page.index === index, typeof page.content === 'string'];
        this.#require(checks.every(Boolean), 'Invalid local snapshot page. ' + page.error);
        return page.content;
    }
    /** @param {string} content @param {import('/models/SnapshotTransfer.ts').SnapshotTransfer} ticket @returns {import('/models/ObservatoryModel.ts').ObservatoryModel} */
    #decode(content, ticket) {
        this.#require(new TextEncoder().encode(content).length === ticket.bytes, 'Incomplete local snapshot.');
        return JSON.parse(content);
    }
    /** @param {boolean} valid @param {string} message @returns {void} */
    #require(valid, message) {
        if (!valid) {
            throw new Error(message);
        }
    }
    /** @returns {import('/services/SnapshotReaderService.js').default} */
    #reader() {
        if (this.snapshotReader === undefined) {
            throw new Error('Required native snapshot provider is unavailable.');
        }
        return this.snapshotReader;
    }
}

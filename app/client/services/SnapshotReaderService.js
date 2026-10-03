@Service
export default class SnapshotReaderService {
    /** @type {string} */
    static _NATIVE_REQUIRED = 'Open the macOS app to read local measurements.';
    /** @param {import('/models/NativeViewSelection.ts').NativeViewSelection} _selection @returns {Promise<import('/models/SnapshotTransfer.ts').SnapshotTransfer>} */
    open(_selection) {
        return Promise.reject(new Error(SnapshotReaderService._NATIVE_REQUIRED));
    }
    /** @param {number} _id @param {number} _index @returns {Promise<import('/models/SnapshotPage.ts').SnapshotPage>} */
    page(_id, _index) {
        return Promise.reject(new Error(SnapshotReaderService._NATIVE_REQUIRED));
    }
    /** @param {number} _id @returns {Promise<boolean>} */
    release(_id) {
        return Promise.reject(new Error(SnapshotReaderService._NATIVE_REQUIRED));
    }
}

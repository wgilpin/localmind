"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
const documentStore_1 = require("./documentStore");
const fsPromises = __importStar(require("fs/promises"));
jest.mock('fs/promises', () => ({
    readFile: jest.fn(),
    writeFile: jest.fn(),
    mkdir: jest.fn(() => Promise.resolve()),
}));
describe('DocumentStoreService', () => {
    let documentStore;
    const filePath = 'test-documents.json';
    beforeEach(() => {
        documentStore = new documentStore_1.DocumentStoreService(filePath);
        jest.clearAllMocks();
    });
    describe('add and get', () => {
        it('should add a document and retrieve it by ID', () => __awaiter(void 0, void 0, void 0, function* () {
            const newDocument = {
                content: 'This is the content of document 1.',
                url: 'http://example.com/doc1',
                title: 'Document 1',
                timestamp: Date.now(),
            };
            const addedDocument = yield documentStore.add(newDocument);
            expect(yield documentStore.get(addedDocument.id)).toEqual(expect.objectContaining(newDocument));
        }));
        it('should return undefined for a non-existent document', () => __awaiter(void 0, void 0, void 0, function* () {
            expect(yield documentStore.get('nonExistentDoc')).toBeUndefined();
        }));
    });
    describe('save', () => {
        it('should attempt to write to the file system with the correct data', () => __awaiter(void 0, void 0, void 0, function* () {
            const docContent1 = 'Content 1.';
            const docContent2 = 'Content 2.';
            const newDoc1 = { content: docContent1, url: 'url1', title: 'title1', timestamp: Date.now() };
            const newDoc2 = { content: docContent2, url: 'url2', title: 'title2', timestamp: Date.now() };
            const addedDoc1 = yield documentStore.add(newDoc1);
            const addedDoc2 = yield documentStore.add(newDoc2);
            yield documentStore.save();
            const expectedData = JSON.stringify([
                Object.assign({ id: addedDoc1.id }, newDoc1),
                Object.assign({ id: addedDoc2.id }, newDoc2),
            ], null, 2);
            expect(fsPromises.writeFile).toHaveBeenCalledWith(filePath, expectedData, 'utf8');
        }));
    });
    describe('load', () => {
        it('should correctly parse data from the mocked file system', () => __awaiter(void 0, void 0, void 0, function* () {
            const mockData = [
                { id: 'docA', content: 'Mocked content A.', url: 'urlA', title: 'titleA', timestamp: 123 },
                { id: 'docB', content: 'Mocked content B.', url: 'urlB', title: 'titleB', timestamp: 456 },
            ];
            fsPromises.readFile.mockResolvedValueOnce(JSON.stringify(mockData));
            yield documentStore.load();
            expect(yield documentStore.get('docA')).toEqual(expect.objectContaining({ content: 'Mocked content A.' }));
            expect(yield documentStore.get('docB')).toEqual(expect.objectContaining({ content: 'Mocked content B.' }));
        }));
        it('should handle file not found gracefully', () => __awaiter(void 0, void 0, void 0, function* () {
            fsPromises.readFile.mockRejectedValueOnce({ code: 'ENOENT' });
            yield expect(documentStore.load()).resolves.not.toThrow();
            expect(yield documentStore.get('anyDoc')).toBeUndefined();
            expect(fsPromises.writeFile).toHaveBeenCalledWith(filePath, '[]', 'utf8');
        }));
        it('should handle invalid JSON gracefully', () => __awaiter(void 0, void 0, void 0, function* () {
            fsPromises.readFile.mockResolvedValueOnce('invalid json');
            // Expect the promise to reject when parsing invalid JSON
            yield expect(documentStore.load()).rejects.toThrow(SyntaxError);
            expect(yield documentStore.get('anyDoc')).toBeUndefined();
        }));
    });
});

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
exports.VectorStoreService = void 0;
const faiss_node_1 = require("faiss-node");
const fs = __importStar(require("fs"));
const config_1 = require("../config");
/**
 * Service for managing a FAISS vector store.
 */
class VectorStoreService {
    constructor(filePath) {
        this.dimension = config_1.OllamaConfig.embeddingDimension;
        this.index = new faiss_node_1.Index(this.dimension);
        this.filePath = filePath;
    }
    /**
     * Returns the file path for the vector store.
     * @returns The file path.
     */
    getFilePath() {
        return this.filePath;
    }
    /**
     * Adds a batch of vectors to the index.
     * @param vectors The vectors to add.
     */
    add(vectors) {
        // faiss-node add expects a 2D array of vectors
        vectors.forEach(vector => this.index.add(vector));
    }
    /**
     * Searches the index for the k nearest neighbors to the queryVector.
     * @param queryVector The vector to query.
     * @param k The number of nearest neighbors to retrieve.
     * @returns A promise that resolves to an object containing indices (I) and distances (D).
     */
    search(queryVector, k) {
        return __awaiter(this, void 0, void 0, function* () {
            const result = this.index.search(queryVector, k);
            return { I: result.labels, D: result.distances };
        });
    }
    /**
     * Saves the index to a file.
     * @param path The path to save the index to.
     */
    save(path) {
        return __awaiter(this, void 0, void 0, function* () {
            const pathToSave = path || this.filePath;
            try {
                yield this.index.write(pathToSave);
                console.log(`FAISS index saved to ${pathToSave}`);
            }
            catch (error) {
                console.error(`Error saving FAISS index to ${pathToSave}:`, error);
                throw error;
            }
        });
    }
    /**
     * Loads the index from a file, creating it if it doesn't exist.
     * @param path The path to load the index from.
     */
    load(path) {
        return __awaiter(this, void 0, void 0, function* () {
            const pathToLoad = path || this.filePath;
            try {
                if (fs.existsSync(pathToLoad)) {
                    this.index = yield faiss_node_1.Index.read(pathToLoad);
                    console.log(`FAISS index loaded from ${pathToLoad}`);
                }
                else {
                    console.log(`FAISS index file not found at ${pathToLoad}. Creating a new index.`);
                    this.index = new faiss_node_1.Index(this.dimension);
                }
            }
            catch (error) {
                console.error(`Error loading FAISS index from ${pathToLoad}:`, error);
                throw error;
            }
        });
    }
}
exports.VectorStoreService = VectorStoreService;

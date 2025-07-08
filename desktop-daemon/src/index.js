"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const express_1 = __importDefault(require("express"));
const config_1 = require("./config");
const ollama_1 = require("./services/ollama");
const vectorStore_1 = require("./services/vectorStore");
const documentStore_1 = require("./services/documentStore");
const rag_1 = require("./services/rag");
const app = (0, express_1.default)();
const port = config_1.ServerConfig.port;
app.use(express_1.default.json()); // Middleware to parse JSON request bodies
let ragService;
function startServer() {
    return __awaiter(this, void 0, void 0, function* () {
        const ollamaService = new ollama_1.OllamaService(config_1.OllamaConfig);
        const vectorStoreService = new vectorStore_1.VectorStoreService(config_1.OllamaConfig.vectorIndexFile);
        const documentStoreService = new documentStore_1.DocumentStoreService(config_1.DocumentStoreConfig.documentStoreFile);
        yield documentStoreService.load();
        yield vectorStoreService.load();
        ragService = new rag_1.RagService(ollamaService, vectorStoreService, documentStoreService);
        app.get('/', (req, res) => {
            res.send('Hello from LocalMind Daemon!');
        });
        app.post('/documents', (req, res) => __awaiter(this, void 0, void 0, function* () {
            try {
                const { title, content, url } = req.body;
                if (!title || !content) {
                    return res.status(400).json({ message: 'Title and content are required.' });
                }
                yield ragService.addDocument({ title, content, url });
                res.status(200).json({ message: 'Document added successfully.' });
            }
            catch (error) {
                console.error('Error adding document:', error);
                res.status(500).json({ message: 'Failed to add document.' });
            }
        }));
        app.post('/search', (req, res) => __awaiter(this, void 0, void 0, function* () {
            try {
                const { query } = req.body;
                if (!query) {
                    return res.status(400).send('Query is required.');
                }
                const result = yield ragService.search(query);
                res.status(200).json({ result });
            }
            catch (error) {
                console.error('Error searching:', error);
                res.status(500).json({ message: 'Failed to perform search.' });
            }
        }));
        app.listen(port, () => {
            console.log(`LocalMind Daemon listening at http://localhost:${port}`);
        });
    });
}
startServer().catch(error => {
    console.error('Failed to start server:', error);
    process.exit(1);
});

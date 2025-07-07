
import express from 'express';
import { OllamaConfig, DocumentStoreConfig, ServerConfig } from './config';
import { OllamaService } from './services/ollama';
import { VectorStoreService } from './services/vectorStore';
import { DocumentStoreService } from './services/documentStore';
import { RagService } from './services/rag';

const app = express();
const port = ServerConfig.port;

app.use(express.json()); // Middleware to parse JSON request bodies

let ragService: RagService;

async function startServer() {
  const ollamaService = new OllamaService(OllamaConfig);
  const vectorStoreService = new VectorStoreService(OllamaConfig.vectorIndexFile);
  const documentStoreService = new DocumentStoreService(DocumentStoreConfig.documentStoreFile);

  await documentStoreService.load();
  await vectorStoreService.load();

  ragService = new RagService(ollamaService, vectorStoreService, documentStoreService);

  app.get('/', (req, res) => {
    res.send('Hello from LocalMind Daemon!');
  });

  app.post('/documents', async (req: any, res: any) => {
    try {
      const { title, content, url } = req.body;
      if (!title || !content) {
        return res.status(400).send('Title and content are required.');
      }
      await ragService.addDocument({ title, content, url });
      res.status(200).send('Document added successfully.');
    } catch (error) {
      console.error('Error adding document:', error);
      res.status(500).send('Failed to add document.');
    }
  });

  app.post('/search', async (req: any, res: any) => {
    try {
      const { query } = req.body;
      if (!query) {
        return res.status(400).send('Query is required.');
      }
      const result = await ragService.search(query);
      res.status(200).json({ result });
    } catch (error) {
      console.error('Error searching:', error);
      res.status(500).send('Failed to perform search.');
    }
  });

  app.listen(port, () => {
    console.log(`LocalMind Daemon listening at http://localhost:${port}`);
  });
}

startServer().catch(error => {
  console.error('Failed to start server:', error);
  process.exit(1);
});

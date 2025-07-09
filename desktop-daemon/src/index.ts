
import express from 'express';
import cors from 'cors';
import fs from 'fs';
import path from 'path';
import { OllamaConfig, DocumentStoreConfig, ServerConfig } from './config';
import { OllamaService } from './services/ollama';
import { VectorStoreService } from './services/vectorStore';
import { DocumentStoreService } from './services/documentStore';
import { RagService } from './services/rag';

const app = express();
const port = ServerConfig.port;

app.use(express.json()); // Middleware to parse JSON request bodies
app.use(cors());

// Serve static files from the search-ui directory
app.use(express.static(path.join(__dirname, '..', 'frontend', 'build')));

let ragService: RagService;

async function startServer() {
  console.log('=== startServer Debug ===');
  console.log('OllamaConfig at startup:', JSON.stringify(OllamaConfig, null, 2));
  console.log('=== End startServer Debug ===');
  
  const dataDir = path.dirname(OllamaConfig.vectorIndexFile);
  if (!fs.existsSync(dataDir)) {
    fs.mkdirSync(dataDir, { recursive: true });
  }
  const ollamaService = new OllamaService(OllamaConfig);
  const vectorStoreService = new VectorStoreService(OllamaConfig.vectorIndexFile);
  const documentStoreService = new DocumentStoreService(DocumentStoreConfig.documentStoreFile);

  await documentStoreService.load();
  await vectorStoreService.load();

  ragService = await RagService.create(ollamaService, vectorStoreService, documentStoreService);


  app.post('/documents', async (req: any, res: any) => {
    try {
      const { title, content, url } = req.body;
      if (!title || !content) {
        return res.status(400).json({ message: 'Title and content are required.' });
      }
      await ragService.addDocument({ title, content, url });
      res.status(200).json({ message: 'Document added successfully.' });
    } catch (error) {
      console.error('Error adding document:', error);
      res.status(500).json({ message: 'Failed to add document.' });
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
      res.status(500).json({ message: 'Failed to perform search.' });
    }
  });

  app.get('/search-stream/:query', async (req: any, res: any) => {
    const query = decodeURIComponent(req.params.query);
    
    if (!query) {
      return res.status(400).send('Query is required.');
    }

    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Headers': 'Cache-Control'
    });

    try {
      const result = await ragService.search(query, (status, message) => {
        res.write(`data: ${JSON.stringify({ status, message })}\n\n`);
      });
      
      res.write(`data: ${JSON.stringify({ status: 'result', result })}\n\n`);
      res.end();
    } catch (error) {
      console.error('Error in search stream:', error);
      res.write(`data: ${JSON.stringify({ status: 'error', message: 'Search failed' })}\n\n`);
      res.end();
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

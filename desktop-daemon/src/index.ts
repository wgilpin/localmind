import express from 'express';
import cors from 'cors';
import fs from 'fs';
import path from 'path';
import { OllamaConfig, DocumentStoreConfig, ServerConfig, loadConfig } from './config';
import { OllamaService } from './services/ollama';
import { VectorStoreService } from './services/vectorStore';
import { DatabaseService } from './services/database';
import { RagService } from './services/rag';

const app = express();
// Load config on server startup
loadConfig();
const port = ServerConfig.port;

app.use(express.json());
app.use(cors());

app.use(express.static(path.join(__dirname, '..', 'frontend', 'build')));

let ragService: RagService;
let databaseService: DatabaseService;
let vectorStoreService: VectorStoreService;

async function startServer() {
  console.log('=== startServer Debug ===');
  console.log('OllamaConfig at startup:', JSON.stringify(OllamaConfig, null, 2));
  console.log('=== End startServer Debug ===');
  
  const dataDir = path.dirname(OllamaConfig.vectorIndexFile);
  if (!fs.existsSync(dataDir)) {
    fs.mkdirSync(dataDir, { recursive: true });
  }
  const ollamaService = new OllamaService(OllamaConfig);
  const dbPath = path.join(DocumentStoreConfig.documentStoreFile, '..', 'localmind.db');
  databaseService = new DatabaseService(dbPath);
  vectorStoreService = new VectorStoreService(
    OllamaConfig.vectorIndexFile,
    databaseService,
    ollamaService,
  );

  await vectorStoreService.init();

  ragService = await RagService.create(ollamaService, vectorStoreService);


  app.post('/documents', async (req: any, res: any) => {
    try {
      const { title, content, url } = req.body;
      if (!title || !content) {
        return res.status(400).json({ message: 'Title and content are required.' });
      }
      await ragService.addDocuments([{ title, content, url }]);
      res.status(200).json({ message: 'Document added successfully.' });
    } catch (error) {
      console.error('Error adding document:', error);
      res.status(500).json({ message: 'Failed to add document.' });
    }
  });

  app.get('/documents/:id', async (req: any, res: any) => {
    try {
      const { id } = req.params;
      if (!id) {
        return res.status(400).send('Document ID is required.');
      }
      const document = databaseService.getDocumentById(id);
      if (!document) {
        return res.status(404).send('Document not found.');
      }
      res.status(200).json(document);
    } catch (error) {
      console.error('Error fetching document:', error);
      res.status(500).json({ message: 'Failed to fetch document.' });
    }
  });

  app.delete('/notes/:id', async (req: any, res: any) => {
    try {
      const { id } = req.params;
      if (!id) {
        return res.status(400).send('Note ID is required.');
      }

      const deleted = await ragService.deleteDocument(id);
      if (deleted) {
        res.status(200).json({ message: 'Note and its vector entry deleted successfully.' });
      } else {
        res.status(404).json({ message: 'Note not found or could not be deleted.' });
      }
    } catch (error) {
      console.error('Error deleting note:', error);
      res.status(500).json({ message: 'Failed to delete note.' });
    }
  });


  app.get('/ranked-chunks/:query', async (req: any, res: any) => {
    try {
      const query = decodeURIComponent(req.params.query);
      if (!query) {
        return res.status(400).send('Query is required.');
      }
      const rankedChunks = await ragService.getRankedChunks(query);
      res.status(200).json({ rankedChunks });
    } catch (error) {
      console.error('Error in ranked chunks search:', error);
      res.status(500).json({ message: 'Failed to perform ranked chunks search.' });
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
      await ragService.searchAndStream(query, (status, message, data) => {
        res.write(`data: ${JSON.stringify({ status, message, ...data })}\n\n`);
      });
      res.end();
    } catch (error) {
      console.error('Error in search stream:', error);
      res.write(`data: ${JSON.stringify({ status: 'error', message: 'Search failed' })}\n\n`);
      res.end();
    }
  });

  app.get('/models', async (req: any, res: any) => {
    try {
      const models = await ollamaService.listModels();
      const currentModel = ollamaService.getCompletionModel();
      res.status(200).json({ models, currentModel });
    } catch (error) {
      console.error('Error listing models:', error);
      res.status(500).send('Failed to list models');
    }
  });

  app.post('/models', async (req: any, res: any) => {
    const { model } = req.body;
    if (!model) {
      return res.status(400).send('Missing model name');
    }
    try {
      await ollamaService.setCompletionModel(model);
      res.status(200).send('Completion model updated');
    } catch (error) {
      console.error('Error setting completion model:', error);
      res.status(500).send('Failed to set completion model');
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

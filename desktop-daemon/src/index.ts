import open from 'open';
import express from "express";
import cors from "cors";
import fs from "fs";
import path from "path";
import {
  OllamaConfig,
  IndexingConfig,
  ServerConfig,
  loadConfig,
  appDataDir, // Imported appDataDir
} from "./config";
import { OllamaService } from "./services/ollama";
import { ChromaStoreService } from "./services/chromaStore";
import { DatabaseService } from "./services/database";
import { RagService } from "./services/rag";
import { YoutubeTranscript } from "youtube-transcript";
import { startBookmarkMonitor } from "./services/bookmarkMonitor";

const app = express();
// Load config on server startup
loadConfig();
const port = ServerConfig.port;

app.use(express.json());
app.use(cors());

app.use(express.static(path.join(__dirname, "..", "frontend", "build")));

let ragService: RagService;
let databaseService: DatabaseService;
let vectorStoreService: ChromaStoreService;
let ollamaService: OllamaService; // Declare ollamaService at a higher scope

async function startServer() {
  console.log("=== startServer Debug ===");
  console.log(
    "OllamaConfig at startup:",
    JSON.stringify(OllamaConfig, null, 2)
  );
  console.log("=== End startServer Debug ===");

  const chromaDir = IndexingConfig.chromaDbPath;
  if (!fs.existsSync(chromaDir)) {
    fs.mkdirSync(chromaDir, { recursive: true });
  }
  ollamaService = new OllamaService(OllamaConfig); // Assign to the higher-scoped variable
  const dbPath = path.join(
    appDataDir, // Changed from DocumentStoreConfig.documentStoreFile
    "localmind.db"
  );
  databaseService = new DatabaseService(dbPath);
  vectorStoreService = new ChromaStoreService(
    IndexingConfig.chromaDbPath,
    databaseService,
    ollamaService
  );

  await vectorStoreService.init();

  ragService = new RagService(
    ollamaService,
    vectorStoreService,
    databaseService
  );

  // Keep track of connected clients for status updates
  const clients: { id: number, res: any }[] = [];
  let clientIdCounter = 0;

  // SSE endpoint for status updates
  app.get("/status-stream", (req, res) => {
    res.writeHead(200, {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      Connection: "keep-alive",
      "Access-Control-Allow-Origin": "*",
    });

    const id = clientIdCounter++;
    clients.push({ id, res });

    req.on("close", () => {
      console.log(`Client ${id} disconnected from status stream.`);
      clients.splice(clients.findIndex(client => client.id === id), 1);
    });

    // Send a "connected" event immediately
    res.write(`data: ${JSON.stringify({ status: "connected", message: "Connected to status stream." })}\n\n`);
  });

  // Function to send status updates to all connected clients
  const sendStatusUpdate = (status: string, message: string, data?: any) => {
    clients.forEach(client => {
      try {
        client.res.write(`data: ${JSON.stringify({ status, message, ...data })}\n\n`);
      } catch (error: unknown) { // Cast error to unknown
        console.error(`Error sending status to client ${client.id}:`, error);
        // Client might have disconnected unexpectedly, remove them
        clients.splice(clients.findIndex(c => c.id === client.id), 1);
      }
    });
  };

  // Start the bookmark monitor
  startBookmarkMonitor(ragService, databaseService, sendStatusUpdate);

  app.post("/documents", async (req: any, res: any) => {
    try {
      let { title, content, url } = req.body;

      if (!title || !content) {
        return res
          .status(400)
          .json({ message: "Title and content are required." });
      }

      if (url && url.includes("youtube.com/watch")) {
        try {
          const transcript = await YoutubeTranscript.fetchTranscript(url);
          if (transcript.length > 0) {
            content = transcript.map((t: { text: any }) => t.text).join(" ");
          }
          // youtube titles can start with a number in brackets - remove it
          title = title.replace(/^\([^)]*\)\s*/, "");
        } catch (youtubeError) {
          console.warn(
            `Could not fetch YouTube transcript for ${url}:`,
            youtubeError
          );
          // Fallback to original content if transcript fetching fails
        }
      }

      await ragService.addDocuments([{ title, content, url }]);
      res.status(200).json({ message: "Document added successfully." });
    } catch (error) {
      console.error("Error adding document:", error);
      res.status(500).json({ message: "Failed to add document." });
    }
  });

  app.get("/documents/:id", async (req: any, res: any) => {
    try {
      const { id } = req.params;
      if (!id) {
        return res.status(400).send("Document ID is required.");
      }
      const document = databaseService.getDocumentById(id);
      if (!document) {
        return res.status(404).send("Document not found.");
      }
      res.status(200).json(document);
    } catch (error) {
      console.error("Error fetching document:", error);
      res.status(500).json({ message: "Failed to fetch document." });
    }
  });

  app.delete("/notes/:id", async (req: any, res: any) => {
    try {
      const { id } = req.params;
      if (!id) {
        return res.status(400).send("Note ID is required.");
      }

      const deleted = await ragService.deleteDocument(id);
      if (deleted) {
        res
          .status(200)
          .json({ message: "Note and its vector entry deleted successfully." });
      } else {
        res
          .status(404)
          .json({ message: "Note not found or could not be deleted." });
      }
    } catch (error) {
      console.error("Error deleting note:", error);
      res.status(500).json({ message: "Failed to delete note." });
    }
  });

  app.put("/notes/:id", async (req: any, res: any) => {
    try {
      const { id } = req.params;
      const { title, content } = req.body;

      if (!id || !title || !content) {
        return res
          .status(400)
          .json({ message: "ID, title, and content are required." });
      }

      const updated = databaseService.updateDocument(id, title, content);
      if (updated) {
        res.status(200).json({ message: "Note updated successfully." });
      } else {
        res
          .status(404)
          .json({ message: "Note not found or could not be updated." });
      }
    } catch (error) {
      console.error("Error updating note:", error);
      res.status(500).json({ message: "Failed to update note." });
    }
  });

  app.get("/ranked-chunks/:query", async (req: any, res: any) => {
    try {
      const query = decodeURIComponent(req.params.query);
      if (!query) {
        return res.status(400).send("Query is required.");
      }
      const cutoff = req.query.cutoff ? parseFloat(req.query.cutoff) : undefined;
      const rankedChunks = await ragService.getRankedChunks(query, cutoff);
      res.status(200).json({ rankedChunks });
    } catch (error) {
      console.error("Error in ranked chunks search:", error);
      res
        .status(500)
        .json({ message: "Failed to perform ranked chunks search." });
    }
  });

  app.get("/recent-notes", async (req: any, res: any) => {
    try {
      const limit = parseInt(req.query.limit as string) || 10;
      const offset = parseInt(req.query.offset as string) || 0;
      const recentNotes = databaseService.getRecentDocuments(limit, offset);
      res.status(200).json(recentNotes);
    } catch (error) {
      console.error("Error fetching recent notes:", error);
      res.status(500).json({ message: "Failed to fetch recent notes." });
    }
  });

  app.get("/search-stream/:query", async (req: any, res: any) => {
    const query = decodeURIComponent(req.params.query);

    if (!query) {
      return res.status(400).send("Query is required.");
    }

    res.writeHead(200, {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache",
      Connection: "keep-alive",
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Headers": "Cache-Control",
    });

    try {
      await ragService.searchAndStream(query, (status, message, data) => {
        res.write(`data: ${JSON.stringify({ status, message, ...data })}\n\n`);
      });
      res.end();
    } catch (error) {
      console.error("Error in search stream:", error);
      res.write(
        `data: ${JSON.stringify({
          status: "error",
          message: "Search failed",
        })}\n\n`
      );
      res.end();
    }
  });

  app.post("/stop-generation", (req: any, res: any) => {
    try {
      ollamaService.stopGeneration();
      res.status(200).send("Generation stopped.");
    } catch (error) {
      console.error("Error stopping generation:", error);
      res.status(500).send("Failed to stop generation.");
    }
  });

  app.post("/log-result-click", (req: any, res: any) => {
    try {
      const { searchTerm, documentId, distance } = req.body;
      const timestamp = new Date().toISOString();
      const logEntry = `${timestamp} | Search: "${searchTerm}" | Document: ${documentId} | Distance: ${distance}\n`;
      
      const logFilePath = path.join(appDataDir, "click_analytics.log");
      fs.appendFileSync(logFilePath, logEntry, 'utf8');
      
      res.status(200).json({ message: "Click logged successfully" });
    } catch (error) {
      console.error("Error logging result click:", error);
      res.status(500).json({ message: "Failed to log click" });
    }
  });

  app.get("/models", async (req: any, res: any) => {
    try {
      const models = await ollamaService.listModels();
      const currentModel = ollamaService.getCompletionModel();
      res.status(200).json({ models, currentModel });
    } catch (error) {
      console.error("Error listing models:", error);
      res.status(500).send("Failed to list models");
    }
  });

  app.post("/models", async (req: any, res: any) => {
    const { model } = req.body;
    if (!model) {
      return res.status(400).send("Missing model name");
    }
    try {
      await ollamaService.setCompletionModel(model);
      res.status(200).send("Completion model updated");
    } catch (error) {
      console.error("Error setting completion model:", error);
      res.status(500).send("Failed to set completion model");
    }
  });

  app.listen(port, () => {
    console.log(`LocalMind Daemon listening at http://localhost:${port}`);
    open(`http://localhost:${port}`); // open the frontend in the default browser
  });
}

startServer().catch((error) => {
  console.error("Failed to start server:", error);
  process.exit(1);
});

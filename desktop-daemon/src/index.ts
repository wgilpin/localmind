import express from "express";
import cors from "cors";
import fs from "fs";
import path from "path";
import {
  OllamaConfig,
  DocumentStoreConfig,
  ServerConfig,
  loadConfig,
} from "./config";
import { OllamaService } from "./services/ollama";
import { VectorStoreService } from "./services/vectorStore";
import { DatabaseService } from "./services/database";
import { RagService } from "./services/rag";
import { YoutubeTranscript } from "youtube-transcript";

const app = express();
// Load config on server startup
loadConfig();
const port = ServerConfig.port;

app.use(express.json());
app.use(cors());

app.use(express.static(path.join(__dirname, "..", "frontend", "build")));

let ragService: RagService;
let databaseService: DatabaseService;
let vectorStoreService: VectorStoreService;
let ollamaService: OllamaService; // Declare ollamaService at a higher scope

async function startServer() {
  console.log("=== startServer Debug ===");
  console.log(
    "OllamaConfig at startup:",
    JSON.stringify(OllamaConfig, null, 2)
  );
  console.log("=== End startServer Debug ===");

  const dataDir = path.dirname(OllamaConfig.vectorIndexFile);
  if (!fs.existsSync(dataDir)) {
    fs.mkdirSync(dataDir, { recursive: true });
  }
  ollamaService = new OllamaService(OllamaConfig); // Assign to the higher-scoped variable
  const dbPath = path.join(
    DocumentStoreConfig.documentStoreFile,
    "..",
    "localmind.db"
  );
  databaseService = new DatabaseService(dbPath);
  vectorStoreService = new VectorStoreService(
    OllamaConfig.vectorIndexFile,
    databaseService,
    ollamaService
  );

  await vectorStoreService.init();

  ragService = new RagService(ollamaService, vectorStoreService, databaseService);

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
          content = transcript.map((t: { text: any }) => t.text).join(" ");

          // youtube titles can start with a number in brackets - remove it
          title = title.replace(/^\([^)]*\)\s*/, '');

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
        return res.status(400).json({ message: "ID, title, and content are required." });
      }

      const updated = databaseService.updateDocument(id, title, content);
      if (updated) {
        res.status(200).json({ message: "Note updated successfully." });
      } else {
        res.status(404).json({ message: "Note not found or could not be updated." });
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
      const rankedChunks = await ragService.getRankedChunks(query);
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
  });
}

startServer().catch((error) => {
  console.error("Failed to start server:", error);
  process.exit(1);
});

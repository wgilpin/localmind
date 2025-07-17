import axios from 'axios';
import { convert } from 'html-to-text';
import { cleanText } from './textProcessor';

const MAX_CONTENT_LENGTH = 10000;

/**
 * Fetches HTML content from a given URL, converts it to plain text,
 * cleans, and truncates it.
 * @param url The URL of the webpage to extract content from.
 * @returns A promise that resolves to the extracted and processed text content,
 *          or an empty string if an error occurs.
 */
export async function extractContentFromUrl(url: string): Promise<string> {
    try {
        const response = await axios.get(url, { timeout: 10000 }); // 10-second timeout
        const html = response.data;

        const text = convert(html, {
            wordwrap: false, // Disable word wrapping
            // You can add more options here based on desired text output
            // For example, to ignore certain elements:
            // selectors: [{ selector: 'img', format: 'skip' }]
        });

        // Convert to lowercase, clean, and then truncate
        const processedText = cleanText(text.toLowerCase()).substring(0, MAX_CONTENT_LENGTH);
        console.log(`Extracted content from ${url}:`, processedText.substring(0, 100) + '...'); // Log first 100 characters for debugging
        return processedText;
    } catch (error) {
        console.error(`Error extracting content from ${url}:`, error instanceof Error ? error.message : error);
        return '';
    }
}
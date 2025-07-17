import axios from 'axios';
import { convert } from 'html-to-text';


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
            wordwrap: false,
            preserveNewlines: false,
            selectors: [
                { selector: 'nav', format: 'skip' },
                { selector: 'header', format: 'skip' },
                { selector: 'footer', format: 'skip' },
                { selector: 'aside', format: 'skip' },
                { selector: '.navigation', format: 'skip' },
                { selector: '.navbar', format: 'skip' },
                { selector: '.menu', format: 'skip' },
                { selector: '.sidebar', format: 'skip' },
                { selector: '.breadcrumb', format: 'skip' },
                { selector: 'button', format: 'skip' },
                { selector: 'input', format: 'skip' },
                { selector: 'select', format: 'skip' },
                { selector: 'form', format: 'skip' },
                { selector: '.btn', format: 'skip' },
                { selector: '.button', format: 'skip' },
                { selector: '.ads', format: 'skip' },
                { selector: '.advertisement', format: 'skip' },
                { selector: '.promo', format: 'skip' },
                { selector: '.banner', format: 'skip' },
                { selector: '.Header', format: 'skip' },
                { selector: '.AppHeader', format: 'skip' },
                { selector: '.js-header-wrapper', format: 'skip' },
                { selector: '.BorderGrid-cell', format: 'skip' },
                { selector: '.file-navigation', format: 'skip' },
                { selector: '.skip-to-content', format: 'skip' },
                { selector: 'a[href="#content"]', format: 'skip' },
                { selector: 'a[href="#main"]', format: 'skip' },
                { selector: 'main', format: 'block' },
                { selector: 'article', format: 'block' },
                { selector: '.content', format: 'block' },
                { selector: '.post-content', format: 'block' },
                { selector: '.readme', format: 'block' },
                { selector: '.markdown-body', format: 'block' },
                { selector: 'a', options: { ignoreHref: true } },
            ]
        });

        // Clean, convert to lowercase, and then truncate
        let processedText = cleanExtractedText(text).toLowerCase().substring(0, MAX_CONTENT_LENGTH);
        
        // If the extracted content indicates JavaScript is not enabled, use the URL as content
        if (processedText.startsWith("javascript isn't enabled in your browser")) {
            processedText = url;
            console.log(`Using URL as content for ${url} due to JavaScript not enabled message.`);
        }

        console.log(`Extracted content from ${url}:`, processedText.substring(0, 100) + '...'); // Log first 100 characters for debugging
        return processedText;
    } catch (error) {
        console.error(`Error extracting content from ${url}:`, error instanceof Error ? error.message : error);
        return '';
    }
}

/**
 * Performs additional cleaning on extracted text to remove common UI patterns and excessive whitespace.
 * @param text The text to clean.
 * @returns The cleaned text.
 */
export function cleanExtractedText(text: string): string {
    return text
        // Remove common navigation patterns
        .replace(/skip to content/gi, '')
        .replace(/navigation menu toggle/gi, '')
        .replace(/sign in|sign up|login/gi, '')
        .replace(/appearance settings/gi, '')
        
        // Remove URL patterns in brackets
        .replace(/\[\/[^\]]+\]/g, '')
        .replace(/\[[^\]]*https?:\/\/[^\]]*\]/g, '')
        
        // Remove repetitive UI text
        .replace(/you must be signed in to/gi, '')
        .replace(/provide feedback/gi, '')
        .replace(/we read every piece of feedback/gi, '')
        
        // Clean up whitespace
        .replace(/\s+/g, ' ')
        .replace(/\n\s*\n/g, '\n')
        .trim();
}
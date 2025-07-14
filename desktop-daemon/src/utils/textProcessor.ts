/**
 * Utility functions for text processing.
 */

/**
 * Cleans and normalizes text content.
 * @param text The input text string.
 * @returns The cleaned and normalized text.
 */
export function cleanText(text: string): string {
    return text.replace(/\s+/g, ' ').trim();
}
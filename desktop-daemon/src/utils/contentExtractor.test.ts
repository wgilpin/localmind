import axios from 'axios';
import { extractContentFromUrl } from './contentExtractor';

jest.mock('axios');
const mockedAxios = axios as jest.Mocked<typeof axios>;

describe('contentExtractor', () => {
    beforeEach(() => {
        mockedAxios.get.mockClear();
    });

    test('should extract and clean content from a URL', async () => {
        const mockHtml = `
            <!DOCTYPE html>
            <html>
            <head><title>Test Page</title></head>
            <body>
                <h1>Hello, World!</h1>
                <p>This is a test paragraph with some <b>bold</b> text.</p>
                <p>   Extra spaces here.   </p>
            </body>
            </html>
        `;
        mockedAxios.get.mockResolvedValueOnce({ data: mockHtml });

        const url = 'http://example.com/test';
        const extractedContent = await extractContentFromUrl(url);

        expect(mockedAxios.get).toHaveBeenCalledWith(url, expect.any(Object));
        expect(extractedContent).toContain('hello, world!');
        expect(extractedContent).toContain('this is a test paragraph with some bold text.');
        expect(extractedContent).not.toContain('<b>');
        expect(extractedContent).not.toContain('Extra spaces here.   ');
        expect(extractedContent).toContain('extra spaces here.');
    });

    test('should truncate content to MAX_CONTENT_LENGTH', async () => {
        const longText = 'a'.repeat(15000);
        const mockHtml = `<html><body>${longText}</body></html>`;
        mockedAxios.get.mockResolvedValueOnce({ data: mockHtml });

        const url = 'http://example.com/long';
        const extractedContent = await extractContentFromUrl(url);

        expect(extractedContent.length).toBe(10000);
        expect(extractedContent).toBe('a'.repeat(10000));
    });

    test('should return empty string on network error (e.g., 404)', async () => {
        mockedAxios.get.mockRejectedValueOnce(new Error('Request failed with status code 404'));

        const url = 'http://example.com/nonexistent';
        const extractedContent = await extractContentFromUrl(url);

        expect(mockedAxios.get).toHaveBeenCalledWith(url, expect.any(Object));
        expect(extractedContent).toBe('');
    });

    test('should return empty string on invalid URL', async () => {
        mockedAxios.get.mockRejectedValueOnce(new Error('Invalid URL'));

        const url = 'invalid-url';
        const extractedContent = await extractContentFromUrl(url);

        expect(mockedAxios.get).toHaveBeenCalledWith(url, expect.any(Object));
        expect(extractedContent).toBe('');
    });
});
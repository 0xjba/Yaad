export type ViewMode = 'recall' | 'capture';
export type CaptureState = 'recording' | 'review';

export interface Memory {
    id: string;
    content: string;
    screenshot_path?: string;
    context_url?: string;
    created_at: string;
}

export interface SearchResult {
    memory: Memory;
    similarity: number;
}

export interface MemoryItem {
    id: string;
    text: string;
    timestamp: string;
    screenshotPath?: string;
    url?: string;
}

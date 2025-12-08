export type ViewMode = 'recall' | 'capture';
export type CaptureState = 'recording' | 'review';

export interface Memory {
    id: number;
    content: string;
    created_at: string;
    embedding?: number[];
    context_url?: string | null;
    context_note?: string | null;
    duration_sec?: number | null;
}

export interface SearchResult {
    memory: Memory;
    similarity: number;
}

export interface MemoryItem {
    id: string;
    title: string;
    preview: string;
    timestamp: string;
    type: string;
    content: string;
}

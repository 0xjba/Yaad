import React, { useState, useEffect } from 'react';
import { Search, Plus } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { formatDistanceToNow } from 'date-fns';
import { MemoryItem, SearchResult } from '../types';

interface RecallViewProps {
  onEscape: () => void;
  onToggleView: () => void;
}

export const RecallView: React.FC<RecallViewProps> = ({ onEscape, onToggleView }) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<MemoryItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  
  // Real Search
  useEffect(() => {
    const doSearch = async () => {
        if (!query.trim()) {
            setResults([]); 
            return;
        }
        try {
            // Rust returns { memory: {id, content, ...}, similarity: 0.9 }
            const raw = await invoke<SearchResult[]>('search_memories', { query, limit: 5 });
            const mapped: MemoryItem[] = raw.map((r) => ({
                id: r.memory.id.toString(),
                title: r.memory.content,
                preview: r.memory.content, // Use content as preview for now
                timestamp: formatDistanceToNow(new Date(r.memory.created_at), { addSuffix: true }),
                type: 'note', // Default type
                content: r.memory.context_note || ''
            }));
            setResults(mapped);
            setSelectedIndex(0); // Reset selection on new results
        } catch (e) {
            console.error(e);
        }
    };
    const timer = setTimeout(doSearch, 150); // Debounce
    return () => clearTimeout(timer);
  }, [query]);

  // Keyboard Navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
            if (query) {
                setQuery('');
                setResults([]);
            } else {
                onEscape();
            }
            return;
        }

        if (results.length > 0) {
            if (e.key === 'ArrowDown') {
                e.preventDefault();
                setSelectedIndex(prev => (prev + 1) % results.length);
            } else if (e.key === 'ArrowUp') {
                e.preventDefault();
                setSelectedIndex(prev => (prev - 1 + results.length) % results.length);
            } else if (e.key === 'Enter') {
                // Action for selection (e.g. copy or expand)
                // For now, no specific action defined in prompt other than "Select"
                // We could maybe log it or just do nothing.
            }
        }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [results, query, onEscape]);

  return (
    <div className="flex flex-col h-full w-full">
      {/* Top Pill */}
      <div className="h-[52px] w-full vibrancy panel-base rounded-xl flex items-center px-3 shrink-0 z-10 gap-3">
        <div className="flex items-center bg-black/20 rounded-lg p-1 border border-white/5 shrink-0 select-none">
             <button onClick={onToggleView} className="flex items-center justify-center w-7 h-7 text-txt-tertiary hover:text-txt-secondary transition-colors">
                 <Plus size={14} />
             </button>
             <div className="flex items-center justify-center w-7 h-7 bg-electric text-white rounded-md shadow-sm ring-1 ring-white/10">
                 <Search size={14} />
             </div>
        </div>
        <input 
          autoFocus
          type="text"
          placeholder="Search memory..."
          className="flex-1 bg-transparent border-none outline-none text-base text-txt-primary placeholder-txt-tertiary font-medium h-full"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      <div className="h-3 shrink-0"></div>

      {/* Results List */}
      <div className="flex-1 vibrancy panel-base rounded-xl overflow-hidden flex flex-col animate-slide-down origin-top">
          <div className="flex-1 overflow-y-auto p-2 min-h-0">
            {results.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full text-txt-tertiary text-sm">
                    {query ? 'No results found' : 'Start typing to recall...'}
                </div>
            ) : (
                results.map((item, index) => (
                    <div 
                        key={item.id}
                        className={`relative flex flex-col rounded-lg cursor-pointer transition-all duration-200 mb-1 p-3 ${
                            index === selectedIndex ? 'bg-white/10' : 'hover:bg-white/5'
                        }`}
                        onClick={() => setSelectedIndex(index)}
                    >
                        <div className="flex justify-between items-start mb-1">
                            <div className={`text-sm font-medium ${index === selectedIndex ? 'text-txt-primary' : 'text-txt-secondary'}`}>
                                {item.title}
                            </div>
                            <span className="text-[10px] text-txt-tertiary whitespace-nowrap ml-2">
                                {item.timestamp}
                            </span>
                        </div>
                    </div>
                ))
            )}
          </div>
          {/* Footer */}
          <div className="px-4 py-3 border-t border-glass-border flex items-center justify-between bg-black/20 text-xs font-medium shrink-0">
             <span className="text-txt-tertiary">{results.length} items</span>
             <div className="flex items-center gap-2 text-txt-secondary">
                <span>Select</span> <span className="bg-neutral-800 px-1.5 py-0.5 rounded border border-white/5 text-[10px]">↵</span>
             </div>
          </div>
      </div>
    </div>
  );
};

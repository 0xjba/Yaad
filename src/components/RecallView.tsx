import React, { useState, useEffect } from 'react';
import { Search, Plus, Copy, Check } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
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
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [isCopied, setIsCopied] = useState(false);
  
  // Real Search
  useEffect(() => {
    let isActive = true;
    const doSearch = async () => {
        if (!query.trim()) {
            if (isActive) setResults([]); 
            return;
        }
        try {
            // Rust returns { memory: {id, content, ...}, similarity: 0.9 }
            const raw = await invoke<SearchResult[]>('search_memories', { query, limit: 5 });
            if (!isActive) return;

            const mapped: MemoryItem[] = raw.map((r) => ({
                id: r.memory.id.toString(),
                text: r.memory.content,
                // Fix: Remove "about " prefix to keep timestamp short
                timestamp: formatDistanceToNow(new Date(r.memory.created_at), { addSuffix: true }).replace('about ', ''),
            }));
            setResults(mapped);
            setSelectedIndex(0); // Reset selection on new results
            setIsCopied(false);
        } catch (e) {
            console.error(e);
        }
    };
    const timer = setTimeout(doSearch, 150); // Debounce
    return () => {
        isActive = false;
        clearTimeout(timer);
    };
  }, [query]);

  const handleCopy = async () => {
    const item = results.find(r => r.id === expandedId);
    if (item) {
        try {
            await writeText(item.text);
            setIsCopied(true);
        } catch (err) {
            console.error('Failed to copy text:', err);
        }
    }
  };

  const handleExpand = () => {
    if (results.length > 0) {
        setExpandedId(results[selectedIndex].id);
        setIsCopied(false);
    }
  };

  // Keyboard Navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
            if (expandedId !== null) {
                setExpandedId(null);
                setIsCopied(false);
            } else if (query) {
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
                setSelectedIndex(prev => {
                    const next = (prev + 1) % results.length;
                    if (expandedId !== null) {
                        setExpandedId(null); 
                        setIsCopied(false);
                    }
                    return next;
                });
            } else if (e.key === 'ArrowUp') {
                e.preventDefault();
                setSelectedIndex(prev => {
                    const next = (prev - 1 + results.length) % results.length;
                    if (expandedId !== null) {
                        setExpandedId(null);
                        setIsCopied(false);
                    }
                    return next;
                });
            } else if (e.key === 'Enter') {
                if (expandedId === null) {
                    handleExpand();
                }
            }
        }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [results, query, onEscape, selectedIndex, expandedId]);

  return (
    <div className="flex flex-col h-full w-full">
      {/* Top Pill */}
      <div className="h-[44px] w-full vibrancy panel-base rounded-xl flex items-center px-3 shrink-0 z-10 gap-3">
        <div className="flex items-center bg-black/5 dark:bg-black/20 rounded-lg p-1 border border-black/10 dark:border-white/10 shrink-0 select-none shadow-inner">
             <button onClick={onToggleView} className="flex items-center justify-center w-6 h-6 text-txt-tertiary hover:text-txt-secondary transition-colors">
                 <Plus size={13} />
             </button>
             <div className="flex items-center justify-center w-6 h-6 bg-electric text-white rounded-md shadow-sm ring-1 ring-white/10">
                 <Search size={13} />
             </div>
        </div>
        <input 
          autoFocus
          type="text"
          placeholder="Search memory..."
          className="flex-1 bg-transparent border-none outline-none text-sm text-txt-primary placeholder-txt-tertiary font-medium h-full"
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
                results.map((item, index) => {
                    const isSelected = index === selectedIndex;
                    const isExpanded = item.id === expandedId;
                    // If any item is expanded, should we hide others? No, user said "in the list... in expanded view".
                    // Implies list stays.
                    
                    return (
                    <div 
                        key={item.id}
                        className={`relative flex flex-col rounded-lg cursor-pointer transition-all duration-200 mb-1 px-3 py-2 ${
                                isSelected ? 'bg-black/10 dark:bg-white/10' : 'hover:bg-black/5 dark:hover:bg-white/5'
                        }`}
                            onClick={() => {
                                setSelectedIndex(index);
                                setExpandedId(item.id);
                                setIsCopied(false);
                            }}
                    >
                            <div className="flex justify-between items-start">
                                <div className={`text-xs font-medium flex-1 ${isExpanded ? 'text-txt-primary whitespace-pre-wrap' : 'text-txt-secondary truncate'}`}>
                                {item.text}
                            </div>
                                <span className="text-[10px] text-txt-tertiary whitespace-nowrap ml-2 shrink-0 pt-0.5">
                                {item.timestamp}
                            </span>
                        </div>
                    </div>
                    );
                })
            )}
          </div>
          {/* Footer */}
          <div className="px-4 py-2 border-t border-glass-border flex items-center justify-between bg-black/5 dark:bg-black/20 text-xs font-medium shrink-0">
             <span className="text-txt-tertiary">{results.length} items</span>
             
             {expandedId !== null ? (
                 <div 
                    className={`flex items-center gap-2 cursor-pointer transition-colors ${
                        isCopied ? 'text-txt-primary' : 'text-txt-secondary hover:text-txt-primary'
                    }`}
                    onClick={handleCopy}
                 >
                    <span>{isCopied ? 'Copied' : 'Copy'}</span> 
                    <span className="bg-neutral-200 dark:bg-neutral-800 p-0.5 rounded border border-black/5 dark:border-white/5 text-[10px]">
                        {isCopied ? <Check size={10} /> : <Copy size={10} />}
                    </span>
                 </div>
             ) : (
             <div className="flex items-center gap-2 text-txt-secondary">
                <span>Select</span> <span className="bg-neutral-200 dark:bg-neutral-800 px-1.5 py-0.5 rounded border border-black/5 dark:border-white/5 text-[10px]">↵</span>
             </div>
             )}
          </div>
      </div>
    </div>
  );
};

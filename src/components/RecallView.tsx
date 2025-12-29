import React, { useState, useEffect } from 'react';
import { Search, Plus, Copy, Check, ExternalLink, Globe, Eye, Monitor } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { appLocalDataDir, join } from '@tauri-apps/api/path';
import { readFile } from '@tauri-apps/plugin-fs'; // <--- Critical Import
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { formatDistanceToNow } from 'date-fns';
import { MemoryItem, SearchResult } from '../types';
import { PillContainer } from './ui/PillContainer';
import { IconButton } from './ui/IconButton';
import { KeyboardBadge } from './ui/KeyboardBadge';

interface RecallViewProps {
  onEscape: () => void;
  onToggleView: () => void;
}

// --- HELPER: Extract Snippet ---
// Only returns text if the query matches something in the OCR
const getContextSnippet = (text: string | undefined, query: string): string | null => {
    if (!text || !query.trim()) return null;
    
    const lowerText = text.toLowerCase();
    const lowerQuery = query.toLowerCase();
    const index = lowerText.indexOf(lowerQuery);
    
    if (index === -1) return null;
    
    // Grab 20 chars before and 40 chars after
    let start = Math.max(0, index - 20);
    let end = Math.min(text.length, index + query.length + 40);
    
    let snippet = text.slice(start, end).trim();
    
    // Add ellipses
    if (start > 0) snippet = "..." + snippet;
    if (end < text.length) snippet = snippet + "...";
    
    return snippet;
};

// --- NEW COMPONENT: Safely loads images using the FS plugin ---
const ImageThumbnail = ({ 
  dir, 
  path, 
  className 
}: { 
  dir: string; 
  path: string; 
  className?: string 
}) => {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    async function load() {
      try {
        const fullPath = await join(dir, path);
        // This works because you allowed "$APP_DATA/screenshots/*" in default.json
        const bytes = await readFile(fullPath); 
        // Create a blob URL (efficient/fast)
        const blob = new Blob([bytes], { type: 'image/jpeg' });
        const url = URL.createObjectURL(blob);
        if (active) setSrc(url);
      } catch (err) {
        console.error("Failed to load image:", err);
      }
    }
    load();
    return () => {
      active = false;
      if (src) URL.revokeObjectURL(src);
    };
  }, [dir, path]);

  if (!src) return <div className="w-full h-full bg-black/5 animate-pulse" />;
  
  return <img src={src} alt="Context" className={className} />;
};
// -------------------------------------------------------------

export const RecallView: React.FC<RecallViewProps> = ({ onEscape, onToggleView }) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<MemoryItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [isCopied, setIsCopied] = useState(false);
  const [isSuggested, setIsSuggested] = useState(false);
  const [screenshotDir, setScreenshotDir] = useState<string | null>(null);

  useEffect(() => {
    appLocalDataDir().then(dir => {
        join(dir, 'screenshots').then(setScreenshotDir);
    });
  }, []);
  
  useEffect(() => {
    if (!query.trim()) {
        invoke<SearchResult[]>('get_contextual_suggestions')
            .then(raw => {
                if (raw.length > 0) {
                                const mapped: MemoryItem[] = raw.map((r) => ({
                                    id: r.memory.id.toString(),
                                    text: r.memory.content,
                                    timestamp: formatDistanceToNow(new Date(r.memory.created_at), { addSuffix: true }).replace('about ', ''),
                                    screenshotPath: r.memory.screenshot_path,
                                    url: r.memory.context_url,
                                    appName: r.memory.app_name,
                                    ocrText: r.memory.ocr_text,
                                }));
                    setResults(mapped);
                    setIsSuggested(true);
                }
            })
            .catch(console.error);
    } else {
        setIsSuggested(false);
    }
  }, [query]);

  useEffect(() => {
    let isActive = true;
    const doSearch = async () => {
        if (!query.trim()) {
            if (isActive) setResults([]); 
            return;
        }
        try {
            const raw = await invoke<SearchResult[]>('search_memories', { query, limit: 5 });
            if (!isActive) return;

            const mapped: MemoryItem[] = raw.map((r) => ({
                id: r.memory.id.toString(),
                text: r.memory.content,
                timestamp: formatDistanceToNow(new Date(r.memory.created_at), { addSuffix: true }).replace('about ', ''),
                screenshotPath: r.memory.screenshot_path,
                url: r.memory.context_url,
                appName: r.memory.app_name,
                ocrText: r.memory.ocr_text,
            }));
            setResults(mapped);
            setSelectedIndex(0);
            setIsCopied(false);
        } catch (e) {
            console.error(e);
        }
    };
    const timer = setTimeout(doSearch, 150);
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
                import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
                    getCurrentWindow().hide();
                });
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
      <PillContainer>
        <div className="flex items-center bg-black/5 dark:bg-black/20 rounded-lg p-1 border border-black/10 dark:border-white/10 shrink-0 select-none shadow-inner">
             <IconButton variant="ghost" onClick={onToggleView} icon={<Plus size={13} />} />
             <IconButton variant="primary" icon={<Search size={13} />} />
        </div>
        <input 
          autoFocus
          type="text"
          placeholder="Search memory..."
          className="flex-1 bg-transparent border-none outline-none text-sm text-txt-primary placeholder-txt-tertiary font-medium h-full"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </PillContainer>

      <div className="h-3 shrink-0"></div>

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
                    
                    // CALCULATE SNIPPET HERE (Only show if expanded + matches)
                    const snippet = isExpanded ? getContextSnippet(item.ocrText, query) : null;

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
                        
                        {/* --- NEW: METATAGS SECTION (App Name + Snippet) --- */}
                        {isExpanded && (item.appName || snippet) && (
                            <div className="mt-2 flex flex-wrap gap-2 animate-slide-down">
                                {/* App Name Bubble */}
                                {item.appName && (
                                    <div className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md text-[10px] font-medium bg-black/5 dark:bg-white/10 text-txt-secondary border border-black/5 dark:border-white/5">
                                        <Monitor size={10} className="opacity-70" />
                                        {item.appName}
                                    </div>
                                )}
                                {/* Snippet Bubble (Only if match found) */}
                                {snippet && (
                                    <div className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md text-[10px] font-medium bg-black/5 dark:bg-white/10 text-txt-secondary border border-black/5 dark:border-white/5" title="Matched text on screen">
                                        <Eye size={10} className="opacity-70" />
                                        <span>"{snippet}"</span>
                                    </div>
                                )}
                            </div>
                        )}
                        {/* -------------------------------------------------- */}
                        
                        {/* --- URL DISPLAY --- */}
                        {item.url && isExpanded && (
                                        <div 
                                            className="mt-2 flex items-center gap-2 text-[10px] text-txt-tertiary hover:text-txt-secondary transition-colors group/url"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                invoke('plugin:shell|open', { path: item.url });
                                            }}
                                        >
                                            <Globe size={10} className="shrink-0" />
                                            <span className="truncate flex-1">{item.url}</span>
                                            <ExternalLink size={10} className="opacity-0 group-hover/url:opacity-100 transition-opacity shrink-0" />
                                        </div>
                                    )}

                                    {/* --- FIXED IMAGE RENDERING --- */}
                        {item.screenshotPath && screenshotDir && isExpanded && (
                            <div className="mt-2 w-full aspect-video rounded-lg overflow-hidden border border-black/10 dark:border-white/10 shadow-sm">
                                <ImageThumbnail 
                                    dir={screenshotDir} 
                                    path={item.screenshotPath} 
                                    className="w-full h-full object-cover"
                                />
                            </div>
                        )}
                        {/* ----------------------------- */}
                        
                    </div>
                    );
                })
            )}
          </div>
          <div className="px-4 py-2 border-t border-glass-border flex items-center justify-between bg-black/5 dark:bg-black/20 text-xs font-medium shrink-0">
             <span className="text-txt-tertiary">
                 {isSuggested ? '✨ Suggested' : `${results.length} items`}
             </span>
             
             {expandedId !== null ? (
                 <div 
                    className={`flex items-center gap-2 cursor-pointer transition-colors ${
                        isCopied ? 'text-txt-primary' : 'text-txt-secondary hover:text-txt-primary'
                    }`}
                    onClick={handleCopy}
                 >
                    <span>{isCopied ? 'Copied' : 'Copy'}</span> 
                    <KeyboardBadge>
                        {isCopied ? <Check size={10} /> : <Copy size={10} />}
                    </KeyboardBadge>
                 </div>
             ) : (
             <div className="flex items-center gap-2 text-txt-secondary">
                <span>Select</span> <KeyboardBadge>↵</KeyboardBadge>
             </div>
             )}
          </div>
      </div>
    </div>
  );
};

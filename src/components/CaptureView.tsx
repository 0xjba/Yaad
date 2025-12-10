import React, { useState, useEffect, useRef } from 'react';
import { X, Search, Plus, Check, Loader2, RotateCcw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { AudioVisualizer } from './Visualizer';
import { CaptureState } from '../types';

interface CaptureViewProps {
  onDiscard: () => void;
  onSave: (text: string) => void;
  state: CaptureState;
  setState: (state: CaptureState) => void;
  onToggleView: () => void;
}

export const CaptureView: React.FC<CaptureViewProps> = ({ onDiscard, onSave, state, setState, onToggleView }) => {
  const [transcript, setTranscript] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [isEditing, setIsEditing] = useState(false); // Track if user is editing
  const [error, setError] = useState<string | null>(null);
  
  const recordingTimerRef = useRef<number | null>(null);
  const autosaveTimerRef = useRef<number | null>(null);
  
  // Auto-start recording on mount
  useEffect(() => {
    if (state === 'recording') {
        invoke('start_recording').catch(err => {
            console.error("Start recording failed:", err);
            setError("Mic not found");
        });
        setIsEditing(false); // Reset edit state
        setError(null);
    }
  }, [state, setState]);

  // Handle Stop & Process
  const handleProcess = async () => {
    setIsProcessing(true);
    setError(null);
    try {
        const text = await invoke<string>('stop_recording');
        setTranscript(text);
        setState('review'); // Only expand on success
    } catch (err) {
        let errorMessage = `Error: ${err}`;
        try {
            // Try to parse the error as JSON
            const errorObj = JSON.parse(err as string);
            if (errorObj.code === "AudioTooQuiet") {
                errorMessage = "Couldn’t hear you. Retry.";
            } else if (errorObj.code === "AudioTooShort") {
                errorMessage = "Recording was too short.";
            } else if (errorObj.message) {
                errorMessage = errorObj.message;
            }
        } catch (e) {
            // Parsing failed, use original error string
            console.error("Failed to parse error JSON:", e);
        }
        
        setError(errorMessage);
        // 🚨 IMPORTANT: Do NOT call setState('review'). 
        // This keeps the App in 'recording' state, maintaining the small Pill size.
    } finally {
        setIsProcessing(false);
    }
  };

  const handleRetry = () => {
      setError(null);
      setTranscript('');
      invoke('start_recording').catch(err => {
          console.error("Retry failed:", err);
          setError("Mic not found");
      });
  };

  // 1. Recording Safety Timer (30s limit)
  useEffect(() => {
    if (state === 'recording' && !error) {
        recordingTimerRef.current = window.setTimeout(() => {
            handleProcess();
        }, 29500);
        return () => {
            if (recordingTimerRef.current) clearTimeout(recordingTimerRef.current);
        };
    }
  }, [state, error]);

  // 2. Auto-Save Logic (10s)
  useEffect(() => {
    // Only run if in review mode and user hasn't clicked to edit
    if (state === 'review' && !isEditing && transcript) {
        autosaveTimerRef.current = window.setTimeout(() => {
            onSave(transcript);
        }, 10000); // 10 seconds
    } else {
        // If editing or state changes, clear the timer
        if (autosaveTimerRef.current) {
            clearTimeout(autosaveTimerRef.current);
            autosaveTimerRef.current = null;
        }
    }

    return () => {
        if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
    };
  }, [state, isEditing, onSave, transcript]);

  // Handle User Interaction (Stops Auto-Save)
  const handleUserEdit = () => {
      setIsEditing(true);
      if (autosaveTimerRef.current) {
          clearTimeout(autosaveTimerRef.current);
          autosaveTimerRef.current = null;
      }
  };

  // Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
        if (state === 'recording') {
            if (e.key === 'Enter') {
                if (error) handleRetry();
                else handleProcess();
            }
            if (e.key === 'Escape') {
                // Discard and CLOSE window
                invoke('cancel_recording').catch(() => {});
                // We don't call onDiscard() because that switches to recall view
                // Instead, we invoke a command to hide window or just let blur handler do it?
                // Actually, the prompt says "it should close the app as expected"
                // The cleanest way is to hide the window directly.
                import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
                    getCurrentWindow().hide();
                });
            }
        } else if (state === 'review') {
            if (e.key === 'Enter' && !e.shiftKey) onSave(transcript);
            if (e.key === 'Escape') onDiscard();
        }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [state, transcript, onSave, onDiscard, error]);

  return (
    <div className="flex flex-col h-full w-full">
      
      {/* Top Pill */}
      <div className="h-[44px] w-full vibrancy panel-base rounded-xl flex items-center justify-between px-3 shrink-0 z-10 gap-3">
           
           <div className="flex items-center bg-black/5 dark:bg-black/20 rounded-lg p-1 border border-black/10 dark:border-white/10 shrink-0 select-none shadow-inner">
                 <div className="flex items-center justify-center w-6 h-6 bg-electric text-white rounded-md shadow-sm ring-1 ring-white/10">
                     <Plus size={13} />
                 </div>
                 <button 
                     onClick={onToggleView}
                     className="flex items-center justify-center w-6 h-6 text-txt-tertiary hover:text-txt-secondary transition-colors"
                 >
                     <Search size={13} />
                 </button>
            </div>

           <div className="flex-1 flex items-center justify-start h-full overflow-hidden mr-[-6px]">
             {error ? (
                // 🚨 Error Message in Pill
                <div className="flex items-center text-red-400 text-xs font-medium tracking-tight pl-2 whitespace-nowrap overflow-hidden text-ellipsis animate-fade-in">
                    {error}
                </div>
             ) : state === 'recording' ? (
                <div className="flex items-center gap-3 w-full h-full">
                    <div className="h-full flex-1 flex items-center">
                        <AudioVisualizer isRecording={!isProcessing} />
                    </div>
                </div>
             ) : (
                <div className="flex items-center text-txt-tertiary text-xs font-medium tracking-tight pl-2 whitespace-nowrap overflow-hidden text-ellipsis">
                    Edit your thought or autosaving in 10s
                </div>
             )}
           </div>

           <div className="flex items-center gap-1 pl-0">
               {(state === 'recording' || error) && (
                   <>
                    <button 
                        onClick={() => {
                            // X button behavior: Discard and Close
                            invoke('cancel_recording').catch(() => {});
                            import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
                                getCurrentWindow().hide();
                            });
                        }} 
                        className="w-7 h-7 flex items-center justify-center rounded-md text-txt-tertiary hover:text-txt-primary hover:bg-black/10 dark:hover:bg-white/10"
                    >
                        <X size={15} />
                    </button>
                    
                    {error ? (
                        // 🚨 Retry Button
                        <button 
                            onClick={handleRetry}
                            className="w-7 h-7 flex items-center justify-center rounded-md transition-all duration-300 bg-black/10 dark:bg-white/10 hover:bg-black/20 dark:hover:bg-white/20 text-white"
                            title="Retry"
                        >
                            <RotateCcw size={13} />
                        </button>
                    ) : (
                        // Normal Process Button
                        <button 
                            onClick={handleProcess}
                            disabled={isProcessing}
                            className={`w-7 h-7 flex items-center justify-center rounded-md transition-all duration-300 bg-black/20 dark:bg-white/20 hover:bg-black/30 dark:hover:bg-white/30 text-white ${isProcessing ? 'cursor-wait' : ''}`}
                        >
                            {isProcessing ? <Loader2 size={15} className="animate-spin" /> : <Check size={15} />}
                        </button>
                    )}
                   </>
               )}
           </div>
      </div>

      {/* Review Card */}
      {state === 'review' && (
        <>
            <div className="h-3 shrink-0"></div>
            <div className="flex-1 vibrancy panel-base rounded-xl flex flex-col animate-slide-down origin-top overflow-hidden relative">
                <div className="flex-1 p-4 overflow-hidden flex flex-col">
                    <textarea
                        value={transcript}
                        onChange={(e) => setTranscript(e.target.value)}
                        onFocus={handleUserEdit} // 🚨 Stop timer on click
                        className="flex-1 w-full bg-transparent resize-none outline-none text-txt-primary text-sm leading-relaxed placeholder-txt-tertiary font-normal"
                        placeholder="Transcript..."
                        autoFocus={false} // Don't steal focus immediately so timer can run
                    />
                </div>
                {/* Footer */}
                <div className="relative px-4 py-2 border-t border-glass-border flex items-center justify-end bg-black/5 dark:bg-black/20 text-xs font-medium overflow-hidden">
                    <div className="flex items-center gap-3 z-10 relative">
                        <button onClick={onDiscard} className="flex items-center gap-2 group cursor-pointer text-txt-secondary hover:text-txt-primary">
                            <span>Discard</span> <span className="bg-neutral-200 dark:bg-neutral-800 px-1.5 py-0.5 rounded border border-black/5 dark:border-white/5 text-[10px]">Esc</span>
                        </button>
                        <div className="w-px h-3 bg-black/10 dark:bg-white/10 mx-1"></div>
                        <button onClick={() => onSave(transcript)} className="flex items-center gap-2 group cursor-pointer text-txt-secondary hover:text-txt-primary">
                            <span>Save</span> <span className="bg-neutral-200 dark:bg-neutral-800 px-1.5 py-0.5 rounded border border-black/5 dark:border-white/5 text-[10px]">↵</span>
                        </button>
                    </div>
                    
                    {/* 🚨 THE 10S YELLOW PROGRESS BAR 🚨 */}
                    {/* Explicit style overrides to ensure visibility */}
                    {!isEditing && (
                        <div 
                            className="absolute bottom-0 left-0 h-[3px] w-full animate-autosave z-20" 
                            style={{ backgroundColor: '#FFC531', opacity: 0.8 }}
                        />
                    )}
                </div>
            </div>
        </>
      )}
    </div>
  );
};

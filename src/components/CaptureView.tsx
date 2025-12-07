import React, { useState, useEffect, useRef } from 'react';
import { X, Search, Plus, Check, Loader2 } from 'lucide-react';
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
  
  const recordingTimerRef = useRef<number | null>(null);
  const autosaveTimerRef = useRef<number | null>(null);
  
  // Auto-start recording on mount
  useEffect(() => {
    if (state === 'recording') {
        invoke('start_recording').catch(err => {
            console.error(err);
            setTranscript("Error: Mic not found");
            setState('review');
        });
        setIsEditing(false); // Reset edit state
    }
  }, [state, setState]);

  // Handle Stop & Process
  const handleProcess = async () => {
    setIsProcessing(true);
    try {
        const text = await invoke<string>('stop_recording');
        setTranscript(text);
        setState('review');
    } catch (err) {
        setTranscript(`Error: ${err}`);
        setState('review');
    } finally {
        setIsProcessing(false);
    }
  };

  // 1. Recording Safety Timer (30s limit)
  useEffect(() => {
    if (state === 'recording') {
        recordingTimerRef.current = window.setTimeout(() => {
            handleProcess();
        }, 29500);
        return () => {
            if (recordingTimerRef.current) clearTimeout(recordingTimerRef.current);
        };
    }
  }, [state]);

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
            if (e.key === 'Enter') handleProcess();
            if (e.key === 'Escape') onDiscard();
        } else if (state === 'review') {
            if (e.key === 'Enter' && !e.shiftKey) onSave(transcript);
            if (e.key === 'Escape') onDiscard();
        }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [state, transcript, onSave, onDiscard]);

  return (
    <div className="flex flex-col h-full w-full">
      
      {/* Top Pill */}
      <div className="h-[52px] w-full vibrancy panel-base rounded-xl flex items-center justify-between px-3 shrink-0 z-10 gap-3">
           
           <div className="flex items-center bg-black/20 rounded-lg p-1 border border-white/5 shrink-0 select-none">
                 <div className="flex items-center justify-center w-7 h-7 bg-electric text-white rounded-md shadow-sm ring-1 ring-white/10">
                     <Plus size={14} />
                 </div>
                 <button 
                     onClick={onToggleView}
                     className="flex items-center justify-center w-7 h-7 text-txt-tertiary hover:text-txt-secondary transition-colors"
                 >
                     <Search size={14} />
                 </button>
            </div>

           <div className="flex-1 flex items-center justify-start h-full overflow-hidden">
             {state === 'recording' ? (
                <div className="flex items-center gap-3 w-full h-full">
                    <div className="h-full flex-1 flex items-center">
                        <AudioVisualizer isRecording={!isProcessing} />
                    </div>
                </div>
             ) : (
                <div className="flex items-center text-txt-tertiary text-sm font-medium pl-2">
                    Review Memory
                </div>
             )}
           </div>

           <div className="flex items-center gap-1 pl-2">
               {state === 'recording' && (
                   <>
                    <button onClick={onDiscard} className="w-8 h-8 flex items-center justify-center rounded-md text-txt-tertiary hover:text-txt-primary hover:bg-white/10">
                        <X size={16} />
                    </button>
                    <button 
                        onClick={handleProcess}
                        disabled={isProcessing}
                        className={`w-8 h-8 flex items-center justify-center rounded-md transition-all duration-300 bg-white/20 hover:bg-white/30 text-white shadow-md ${isProcessing ? 'cursor-wait' : ''}`}
                    >
                        {isProcessing ? <Loader2 size={16} className="animate-spin" /> : <Check size={16} />}
                    </button>
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
                <div className="relative px-4 py-3 border-t border-glass-border flex items-center justify-end bg-black/20 text-xs font-medium overflow-hidden">
                    <div className="flex items-center gap-3 z-10 relative">
                        <button onClick={onDiscard} className="flex items-center gap-2 group cursor-pointer text-txt-secondary hover:text-txt-primary">
                            <span>Discard</span> <span className="bg-neutral-800 px-1.5 py-0.5 rounded border border-white/5 text-[10px]">Esc</span>
                        </button>
                        <div className="w-px h-3 bg-white/10 mx-1"></div>
                        <button onClick={() => onSave(transcript)} className="flex items-center gap-2 group cursor-pointer text-txt-secondary hover:text-txt-primary">
                            <span>Save</span> <span className="bg-neutral-800 px-1.5 py-0.5 rounded border border-white/5 text-[10px]">↵</span>
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

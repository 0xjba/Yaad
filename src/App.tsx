import React, { useState, useEffect, useLayoutEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { RecallView } from './components/RecallView';
import { CaptureView } from './components/CaptureView';
import { ViewMode, CaptureState } from './types';

export default function App() {
  const [view, setView] = useState<ViewMode | null>(null);
  const [captureState, setCaptureState] = useState<CaptureState>('recording');
  
  // --- EXACT DIMENSIONS (Synced with Main.rs) ---
  const WINDOW_WIDTH = 320; 
  const HEIGHT_PILL = 44;     // Exactly 44px
  const HEIGHT_REVIEW = 200;  // 200px content
  const HEIGHT_RECALL = 280;  // 280px content

  useEffect(() => {
    invoke('initialize_app').catch(console.error);
  }, []);

  // --- RESIZER LOGIC ---
  // Replaced setTimeout with useLayoutEffect to prevent ghost windows
  useLayoutEffect(() => {
    if (!view) return;

    const resizeWindow = async () => {
      const appWindow = getCurrentWindow();
      let targetHeight = HEIGHT_RECALL;

      if (view === 'capture') {
        targetHeight = captureState === 'recording' ? HEIGHT_PILL : HEIGHT_REVIEW;
      } else {
        targetHeight = HEIGHT_RECALL;
      }

      try {
        // Immediate resize request
        await appWindow.setSize(new LogicalSize(WINDOW_WIDTH, targetHeight));
      } catch (e) {
        console.error("Resize error:", e);
      }
    };

    resizeWindow();
  }, [view, captureState]);

  // --- LISTENERS ---
  useEffect(() => {
    const unlistenView = listen<string>('set-view', (event) => {
      const newView = event.payload as ViewMode;
      if (newView === 'capture') {
        setCaptureState('recording');
        setView('capture');
      } else {
        setView('recall');
      }
    });
    
    // Handle window blur (user clicks outside)
    const unlistenBlur = listen('window-blur', () => {
        // Discard recording if it was in progress
        invoke('cancel_recording').catch(() => {});
        // Reset state so next open starts fresh
        setCaptureState('recording'); 
    });

    return () => { 
        unlistenView.then(f => f()); 
        unlistenBlur.then(f => f());
    };
  }, []);

  useEffect(() => {
    if (view === 'recall') {
        setCaptureState('recording');
        invoke('cancel_recording').catch(() => {});
    }
  }, [view]);

  // Reset Logic
  const handleToggleView = () => {
      if (view === 'recall') {
          setCaptureState('recording');
          invoke('cancel_recording').catch(() => {});
          setView('capture');
      } else {
          setView('recall');
      }
  };

  // Render Nothing if view is unknown
  if (!view) return null;

  const contentHeightClass = (() => {
      if (view === 'capture') {
          return captureState === 'recording' ? 'h-[44px]' : 'h-[200px]';
      }
      return 'h-[280px]';
  })();

  return (
    <div className="w-full h-full overflow-hidden">
        <div 
            className={`w-full ${contentHeightClass} flex flex-col transition-all duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] relative`}
        >
            {view === 'recall' ? (
                <RecallView 
                    onEscape={() => invoke('cancel_recording')} 
                    onToggleView={handleToggleView}
                />
            ) : (
                <CaptureView 
                    state={captureState}
                    setState={setCaptureState}
                    onDiscard={() => {
                        invoke('cancel_recording').catch(() => {});
                        setView('recall');
                    }} 
                    onSave={async (text) => {
                        await invoke('save_memory', { 
                            content: text, 
                            durationSec: null, 
                            contextUrl: null, 
                            contextNote: null 
                        });
                        setView('recall');
                    }}
                    onToggleView={handleToggleView}
                />
            )}
        </div>
    </div>
  );
}

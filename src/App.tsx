import { useState, useEffect, useLayoutEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { RecallView } from './components/RecallView';
import { CaptureView } from './components/CaptureView';
import { ViewMode, CaptureState } from './types';
import { WINDOW_WIDTH, HEIGHT_PILL, HEIGHT_REVIEW, HEIGHT_RECALL } from './constants';

export default function App() {
  const [view, setView] = useState<ViewMode | null>(null);
  const [captureState, setCaptureState] = useState<CaptureState>('recording');
  const [captureSessionId, setCaptureSessionId] = useState(0);

  useEffect(() => {
    invoke('initialize_app').catch(console.error);
  }, []);

  // --- RESIZER LOGIC ---
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
        setCaptureSessionId(id => id + 1);
      } else {
        setView('recall');
      }
    });
    
    const unlistenBlur = listen('window-blur', () => {
        invoke('cancel_recording').catch(() => {});
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

  const handleToggleView = () => {
      if (view === 'recall') {
          setCaptureState('recording');
          invoke('cancel_recording').catch(console.error);
          setView('capture');
          setCaptureSessionId(id => id + 1);
      } else {
          setView('recall');
      }
  };

  // --- FIXED SAVE HANDLER ---
  const handleSave = useCallback(async (text: string, capture: any) => {
    // 1. Optimistic Hide: Close immediately for snappy UX
    const win = getCurrentWindow();
    await win.hide();

    try {
        console.log("Saving memory...", { text, hasCapture: !!capture });
        
        // 2. Process in Background (Window is already hidden)
        await invoke('save_memory', { 
            content: text, 
            ocrText: capture?.ocr || null,
            appName: capture?.app_name || null,
            screenshot: capture?.image || null,
            durationSec: null, 
            contextUrl: capture?.url || null, 
            contextNote: null 
        });

        // 3. Reset View for next time
        setView('recall');
    } catch (err) {
        console.error("Save failed:", err);
        // Note: We don't alert() here because the window is hidden.
        // Failing silently (logging) is better than a zombie window.
    }
  }, []);

  if (!view) return null;

  const contentHeightClass = (() => {
      if (view === 'capture') {
          return captureState === 'recording' ? 'h-pill' : 'h-review';
      }
      return 'h-recall';
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
                    key={captureSessionId}
                    state={captureState}
                    setState={setCaptureState}
                    onDiscard={async () => {
                        invoke('cancel_recording').catch(() => {});
                        await getCurrentWindow().hide();
                        setView('recall');
                    }} 
                    onSave={handleSave}
                    onToggleView={handleToggleView}
                />
            )}
        </div>
    </div>
  );
}

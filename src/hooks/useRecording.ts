import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CaptureState } from '../types';

interface UseRecordingOptions {
  onSave: (text: string) => void;
  state: CaptureState;
  setState: (state: CaptureState) => void;
}

export const useRecording = ({ onSave, state, setState }: UseRecordingOptions) => {
  const [transcript, setTranscript] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const recordingTimerRef = useRef<number | null>(null);
  const autosaveTimerRef = useRef<number | null>(null);

  // Auto-start recording when state is 'recording'
  useEffect(() => {
    if (state === 'recording') {
      invoke('start_recording').catch(err => {
        console.error("Start recording failed:", err);
        setError("Mic not found");
      });
      setIsEditing(false);
      setError(null);
    }
  }, [state]);

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
          errorMessage = "Couldn't hear you. Retry.";
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
      // Do NOT call setState('review'). This keeps the App in 'recording' state.
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

  // Recording Safety Timer (30s limit)
  useEffect(() => {
    if (state === 'recording' && !error) {
      recordingTimerRef.current = window.setTimeout(() => {
        handleProcess();
      }, 29500);
      return () => {
        if (recordingTimerRef.current) clearTimeout(recordingTimerRef.current);
      };
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state, error]);

  // Auto-Save Logic (10s)
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

  const updateTranscript = (text: string) => {
    setTranscript(text);
  };

  const reset = () => {
    setTranscript('');
    setState('recording');
    setIsEditing(false);
    setError(null);
    if (recordingTimerRef.current) clearTimeout(recordingTimerRef.current);
    if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
  };

  return {
    transcript,
    isProcessing,
    isEditing,
    error,
    handleProcess,
    handleRetry,
    handleUserEdit,
    updateTranscript,
    reset,
  };
};

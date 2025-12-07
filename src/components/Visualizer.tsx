import React, { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

export const AudioVisualizer: React.FC<{ isRecording: boolean }> = ({ isRecording }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  
  // Use a Ref to store the latest amplitude to avoid re-renders
  const amplitudeRef = useRef(0);
  // Ref for smooth transition (Linear Interpolation)
  const smoothedRef = useRef(0);

  // Listen for audio-level events from Rust
  useEffect(() => {
    if (!isRecording) {
      amplitudeRef.current = 0;
      smoothedRef.current = 0;
      return;
    }

    const unlisten = listen<number>('audio-level', (event) => {
       // Clamp value 0.0 to 1.0 (just in case)
       // Amplify slightly for visual impact (x2.5)
       const val = Math.min(Math.max(event.payload, 0), 1.0) * 2.5;
       amplitudeRef.current = val;
    });

    return () => {
       unlisten.then(f => f());
    };
  }, [isRecording]);

  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;

    if (!canvas || !container) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let animationId: number;
    let phase = 0;

    // Handle high-DPI displays for sharp lines
    const updateSize = () => {
      const dpr = window.devicePixelRatio || 1;
      const rect = container.getBoundingClientRect();
      
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      
      ctx.scale(dpr, dpr);
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = `${rect.height}px`;
    };

    // Initial size update
    updateSize();
    window.addEventListener('resize', updateSize);

    const draw = () => {
      if (!container || !ctx) return;
      
      const width = container.clientWidth;
      const height = container.clientHeight;
      const centerY = height / 2;

      ctx.clearRect(0, 0, width, height);
      
      ctx.beginPath();
      // CHANGE THIS: Use a slight white-gray color
      ctx.strokeStyle = '#E0E0E0'; 
      ctx.lineWidth = 2;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      
      // CHANGE THIS: Glow color should match but be transparent
      ctx.shadowBlur = 15; // Increased glow
      ctx.shadowColor = 'rgba(255, 255, 255, 0.4)'; // White at 40% opacity

      if (isRecording) {
        // Linear Interpolation (Lerp) for smoothness
        // Move 15% towards target per frame
        const target = amplitudeRef.current;
        smoothedRef.current += (target - smoothedRef.current) * 0.15;
        
        // Ensure we don't draw a dead flat line even if silent
        const effectiveAmp = Math.max(smoothedRef.current, 0.05);

        // Max visual height (pixels)
        const maxPixelHeight = height / 2 - 4;
        
        for (let x = 0; x <= width; x++) {
          // Normalize x from -1 to 1 for envelope calculation
          const nx = (x / width) * 2 - 1;
          
          // Bell-curve envelope to taper amplitude at the edges (0 at ends, 1 in center)
          const envelope = Math.exp(-4 * nx * nx); 
          
          // Combine two sine waves for organic look
          const wave = (Math.sin(x * 0.08 + phase) * 0.6 + Math.sin(x * 0.15 - phase * 1.2) * 0.4);
          
          const y = centerY + wave * (effectiveAmp * maxPixelHeight) * envelope;

          if (x === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        phase += 0.15; // Animation speed
      } else {
        // Draw flat line when idle
        ctx.moveTo(0, centerY);
        ctx.lineTo(width, centerY);
        phase = 0;
        smoothedRef.current = 0;
        amplitudeRef.current = 0;
      }
      
      ctx.stroke();
      animationId = requestAnimationFrame(draw);
    };

    draw();

    return () => {
      window.removeEventListener('resize', updateSize);
      cancelAnimationFrame(animationId);
    };
  }, [isRecording]);

  return (
    <div ref={containerRef} className="w-full h-full flex items-center justify-center overflow-hidden">
      <canvas ref={canvasRef} className="block" />
    </div>
  );
};

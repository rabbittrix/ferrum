'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

type Props = {
  cwd?: string | null;
};

export function TerminalPanel({ cwd }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const inputBuffer = useRef('');
  const sessionId = useRef(`term-${Date.now()}`);
  const running = useRef(false);
  const [binaryPath, setBinaryPath] = useState<string>('ferrum');
  const [hint, setHint] = useState<string | null>(null);

  const execLine = useCallback(
    async (line: string) => {
      if (running.current || !line.trim()) return;
      running.current = true;
      setHint(null);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke<number>('ferrum_terminal_exec', {
          sessionId: sessionId.current,
          commandLine: line,
          cwd: cwd ?? null,
        });
      } catch (e) {
        termRef.current?.writeln(`\r\n\x1b[31m${String(e)}\x1b[0m`);
        running.current = false;
      }
    },
    [cwd]
  );

  useEffect(() => {
    async function resolveBinary() {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const path = await invoke<string>('ferrum_terminal_binary_path');
        setBinaryPath(path);
      } catch {
        /* web preview */
      }
    }
    resolveBinary();
  }, []);

  useEffect(() => {
    if (!containerRef.current || termRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      fontFamily: '"Cascadia Code", "Fira Code", Consolas, monospace',
      fontSize: 13,
      theme: {
        background: '#0a0e17',
        foreground: '#94a3b8',
        cursor: '#00d4ff',
        selectionBackground: '#1e293b',
      },
      convertEol: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(containerRef.current);
    fit.fit();
    termRef.current = term;
    fitRef.current = fit;

    term.writeln('\x1b[36mFerrum Integrated Terminal\x1b[0m');
    term.writeln(`CLI: \x1b[33m${binaryPath}\x1b[0m — type \x1b[32mferrum doctor\x1b[0m, \x1b[32mferrum plan\x1b[0m, etc.`);
    term.write('\r\n$ ');

    term.onData((data) => {
      if (running.current) return;
      const t = termRef.current;
      if (!t) return;

      for (const ch of data) {
        const code = ch.charCodeAt(0);
        if (code === 13) {
          t.writeln('');
          const line = inputBuffer.current;
          inputBuffer.current = '';
          void execLine(line);
          if (!running.current) t.write('$ ');
        } else if (code === 127) {
          if (inputBuffer.current.length > 0) {
            inputBuffer.current = inputBuffer.current.slice(0, -1);
            t.write('\b \b');
          }
        } else if (code >= 32) {
          inputBuffer.current += ch;
          t.write(ch);
        }
      }
    });

    const onResize = () => fit.fit();
    window.addEventListener('resize', onResize);

    return () => {
      window.removeEventListener('resize', onResize);
      term.dispose();
      termRef.current = null;
    };
  }, [binaryPath, execLine]);

  useEffect(() => {
    let unlistenOut: (() => void) | undefined;
    let unlistenExit: (() => void) | undefined;

    async function setup() {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlistenOut = await listen<{ session_id: string; data: string; stream: string }>(
          'terminal-output',
          (event) => {
            if (event.payload.session_id !== sessionId.current) return;
            termRef.current?.write(event.payload.data);
          }
        );
        unlistenExit = await listen<{ session_id: string; code: number }>(
          'terminal-exit',
          (event) => {
            if (event.payload.session_id !== sessionId.current) return;
            running.current = false;
            const code = event.payload.code;
            if (code !== 0) {
              termRef.current?.writeln(`\x1b[31m[exit ${code}]\x1b[0m`);
            }
            termRef.current?.write('\r\n$ ');
          }
        );
      } catch {
        setHint('Run inside the Ferrum desktop app for live CLI streaming.');
      }
    }
    setup();
    return () => {
      unlistenOut?.();
      unlistenExit?.();
    };
  }, []);

  return (
    <div className="flex h-[calc(100vh-8rem)] flex-col rounded-lg border border-space-700 bg-space-950">
      <div className="flex items-center justify-between border-b border-space-700 px-4 py-2">
        <h2 className="font-display text-lg text-cyan-neon">Integrated Terminal</h2>
        <span className="text-xs text-slate-500">Streams ANSI output from ferrum-cli</span>
      </div>
      {hint && (
        <div className="border-b border-amber-500/30 bg-amber-500/10 px-4 py-2 text-sm text-amber-200">
          {hint}
        </div>
      )}
      <div ref={containerRef} className="min-h-0 flex-1 p-2" />
    </div>
  );
}

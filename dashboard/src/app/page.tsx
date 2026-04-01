'use client';

import { useState, useEffect } from 'react';
import { Shield, BrainCircuit, Activity, CheckCircle2, Hexagon, Database, Lock, AlertCircle } from 'lucide-react';

export default function Home() {
  const [mounted, setMounted] = useState(false);
  const [score, setScore] = useState(0);

  useEffect(() => {
    setMounted(true);
    // Simulate scoring engine calculating credibility
    const interval = setInterval(() => {
      setScore(prev => {
        if (prev >= 94) {
          clearInterval(interval);
          return 94;
        }
        return prev + 1;
      });
    }, 40);
    return () => clearInterval(interval);
  }, []);

  if (!mounted) return null;

  return (
    <main className="min-h-screen relative overflow-hidden flex flex-col items-center justify-start pt-20 pb-24">
      
      {/* Dynamic Background */}
      <div className="absolute top-0 left-0 w-full h-full overflow-hidden -z-10 bg-[#0A0A0B]">
        <div className="absolute top-[-10%] left-[-10%] w-[50%] h-[50%] rounded-full bg-aether-900/40 blur-[120px] mix-blend-screen animate-pulse"></div>
        <div className="absolute bottom-[-20%] right-[-10%] w-[60%] h-[60%] rounded-full bg-aether-600/20 blur-[150px] mix-blend-screen animate-glow"></div>
        <div className="absolute top-[20%] right-[10%] w-[30%] h-[30%] rounded-full bg-[#1E1B4B]/50 blur-[100px] mix-blend-screen"></div>
      </div>

      <div className="w-full max-w-6xl px-6 flex flex-col gap-12 z-10">
        
        {/* Header Section */}
        <div className="flex flex-col items-center text-center space-y-6 max-w-3xl mx-auto">
          <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full glass-panel text-aether-500 font-mono text-sm uppercase tracking-widest animate-glow">
            <Shield className="w-4 h-4" />
            <span>Formally Verified Escrow</span>
          </div>
          <h1 className="text-5xl md:text-7xl font-extrabold tracking-tight text-transparent bg-clip-text bg-gradient-to-br from-white via-slate-200 to-aether-500">
            AETHER <br />
            <span className="text-4xl md:text-5xl font-light tracking-wide text-slate-400">Quantum Vault</span>
          </h1>
          <p className="text-lg text-slate-400 leading-relaxed max-w-2xl">
            The first trustless creator economy infrastructure powered by Lean 4 mathematical proofs, multi-tier AI inference, and Lyapunov-stabilized dynamic pricing.
          </p>
        </div>

        {/* Dashboard Grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 w-full pt-8">
          
          {/* Engine Status Card */}
          <div className="col-span-1 md:col-span-2 glass-panel p-8 rounded-3xl flex flex-col justify-between group hover:border-aether-500/50 transition-colors duration-500 relative overflow-hidden">
            <div className="absolute top-0 right-0 w-64 h-64 bg-aether-600/10 rounded-full blur-[80px] group-hover:bg-aether-500/20 transition-all duration-700"></div>
            
            <div className="flex items-start justify-between z-10">
              <div>
                <h2 className="text-2xl font-bold flex items-center gap-3">
                  <BrainCircuit className="text-aether-500 w-6 h-6" />
                  Epsilon Engine
                </h2>
                <p className="text-sm text-slate-400 mt-2 font-mono">Status: ACTIVE • Tiers: 3</p>
              </div>
              <div className="flex items-center gap-2">
                <span className="relative flex h-3 w-3">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-3 w-3 bg-green-500"></span>
                </span>
                <span className="text-xs uppercase tracking-wider font-semibold text-green-400">Syncing Inference</span>
              </div>
            </div>

            <div className="mt-8 grid grid-cols-3 gap-4 z-10">
              <div className="flex flex-col p-4 bg-slate-800/40 rounded-2xl border border-slate-700/50">
                <span className="text-xs text-slate-400 uppercase tracking-wider font-semibold">Tier 1: Pico</span>
                <span className="text-xl font-mono mt-1 text-white">TinyGrad GPU</span>
              </div>
              <div className="flex flex-col p-4 bg-slate-800/40 rounded-2xl border border-slate-700/50">
                <span className="text-xs text-slate-400 uppercase tracking-wider font-semibold">Tier 2: LogicGate</span>
                <span className="text-xl font-mono mt-1 text-aether-100">Llama.cpp</span>
              </div>
              <div className="flex flex-col p-4 bg-slate-800/40 rounded-2xl border border-slate-700/50">
                <span className="text-xs text-slate-400 uppercase tracking-wider font-semibold">Tier 3: The Architect</span>
                <span className="text-xl font-mono mt-1 text-aether-500">AirLLM 70B</span>
              </div>
            </div>
          </div>

          {/* Realtime Authenticity Score */}
          <div className="col-span-1 glass-panel p-8 rounded-3xl flex flex-col items-center justify-center text-center relative hover:scale-[1.02] transition-transform duration-500">
             <div className="relative w-48 h-48 flex items-center justify-center">
                <svg className="absolute w-full h-full -rotate-90 animate-spin-slow" viewBox="0 0 100 100">
                  <circle cx="50" cy="50" r="45" fill="none" stroke="rgba(99, 102, 241, 0.2)" strokeWidth="2" strokeDasharray="10 5" />
                </svg>
                <div className="absolute inset-0 border-4 border-aether-900 rounded-full"></div>
                
                {/* Score Circular Progress Simulation */}
                <svg className="absolute inset-0 w-full h-full -rotate-90" viewBox="0 0 100 100">
                  <circle 
                    cx="50" cy="50" r="45" 
                    fill="none" 
                    stroke="url(#gradient)" 
                    strokeWidth="4" 
                    strokeDasharray={`${score * 2.8} 280`} 
                    strokeLinecap="round"
                    className="transition-all duration-300 ease-out"
                  />
                  <defs>
                    <linearGradient id="gradient" x1="0%" y1="0%" x2="100%" y2="100%">
                      <stop offset="0%" stopColor="#818cf8" />
                      <stop offset="100%" stopColor="#4f46e5" />
                    </linearGradient>
                  </defs>
                </svg>

                <div className="flex flex-col items-center z-10">
                  <span className="text-5xl font-black text-white">{score}</span>
                  <span className="text-xs text-aether-500 uppercase font-bold tracking-widest mt-1">Authenticity</span>
                </div>
             </div>
          </div>

          {/* Core Invariants Checker */}
          <div className="col-span-1 md:col-span-3 glass-panel p-8 rounded-3xl overflow-hidden relative">
            <h3 className="text-lg font-bold flex items-center gap-2 mb-6">
              <Lock className="w-5 h-5 text-aether-500" />
              Verified Invariants (Lean 4)
            </h3>
            <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-4">
              {[
                "INV-01: No Premature Release",
                "INV-02: No Double Spend",
                "INV-03: Total Conservation",
                "INV-04: Fraud Enforcement",
                "INV-05: Timeout Guarantee",
                "INV-06: Strict Transition"
              ].map((inv, idx) => (
                <div key={idx} className="flex flex-col items-center justify-center p-4 bg-[#0A0A0B]/50 rounded-xl border border-white/5 h-32 hover:bg-aether-900/30 transition-colors">
                  <CheckCircle2 className="w-8 h-8 text-green-500 mb-3" />
                  <span className="text-xs text-center font-mono text-slate-300">{inv}</span>
                </div>
              ))}
            </div>
          </div>

        </div>

        {/* Footer info */}
        <div className="flex items-center justify-between text-xs font-mono text-slate-500 pt-8 border-t border-white/10">
          <div className="flex items-center gap-4">
            <span className="flex items-center gap-1"><Database className="w-3 h-3"/> IPFS Synced</span>
            <span className="flex items-center gap-1"><Hexagon className="w-3 h-3"/> CUDA Active</span>
          </div>
          <span>Rust Platform Engine v1.0 • Epsilon Architecture</span>
        </div>

      </div>
    </main>
  );
}

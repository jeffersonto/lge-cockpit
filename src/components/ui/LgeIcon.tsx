interface LgeIconProps {
  size?: number;
  className?: string;
}

export function LgeIcon({ size = 32, className = "" }: LgeIconProps) {
  const id = `lge-${Math.random().toString(36).slice(2, 7)}`;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 512 512"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <defs>
        <radialGradient id={`bg-${id}`} cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#1a1030" />
          <stop offset="100%" stopColor="#0a0812" />
        </radialGradient>
        <linearGradient id={`top1-${id}`} x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#6d28d9" />
          <stop offset="100%" stopColor="#7c3aed" />
        </linearGradient>
        <linearGradient id={`top2-${id}`} x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#8b5cf6" />
          <stop offset="100%" stopColor="#a855f7" />
        </linearGradient>
        <linearGradient id={`top3-${id}`} x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#a855f7" />
          <stop offset="100%" stopColor="#c084fc" />
        </linearGradient>
        <linearGradient id={`top4-${id}`} x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#c084fc" />
          <stop offset="100%" stopColor="#e879f9" />
        </linearGradient>
        <filter id={`glow-${id}`} x="-30%" y="-30%" width="160%" height="160%">
          <feGaussianBlur stdDeviation="8" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <filter id={`apex-${id}`} x="-100%" y="-100%" width="300%" height="300%">
          <feGaussianBlur stdDeviation="16" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      {/* Background */}
      <circle cx="256" cy="256" r="256" fill={`url(#bg-${id})`} />
      <circle cx="256" cy="256" r="238" fill="none" stroke="#7c3aed" strokeWidth="1.5" opacity="0.3" />

      {/* Layer 1 — bottom */}
      <polygon points="256,345 356,315 356,287 256,317" fill="#2e0a52" opacity="0.95" />
      <polygon points="256,345 156,315 156,287 256,317" fill="#3b0764" opacity="0.95" />
      <polygon points="256,317 356,287 256,257 156,287" fill={`url(#top1-${id})`} opacity="0.95" />
      <polyline points="156,287 256,257 356,287" fill="none" stroke="#a855f7" strokeWidth="1.5" opacity="0.6" />

      {/* Layer 2 */}
      <polygon points="256,292 331,265 331,238 256,265" fill="#4c1087" opacity="0.95" />
      <polygon points="256,292 181,265 181,238 256,265" fill="#5b21b6" opacity="0.95" />
      <polygon points="256,265 331,238 256,211 181,238" fill={`url(#top2-${id})`} opacity="0.95" />
      <polyline points="181,238 256,211 331,238" fill="none" stroke="#c084fc" strokeWidth="1.5" opacity="0.6" />

      {/* Layer 3 */}
      <polygon points="256,238 316,215 316,189 256,212" fill="#6d28d9" opacity="0.95" />
      <polygon points="256,238 196,215 196,189 256,212" fill="#7c3aed" opacity="0.95" />
      <polygon points="256,212 316,189 256,166 196,189" fill={`url(#top3-${id})`} opacity="0.95" />
      <polyline points="196,189 256,166 316,189" fill="none" stroke="#d946ef" strokeWidth="1.5" opacity="0.7" />

      {/* Layer 4 — top */}
      <polygon points="256,185 303,164 303,140 256,161" fill="#9333ea" opacity="0.95" />
      <polygon points="256,185 209,164 209,140 256,161" fill="#a855f7" opacity="0.95" />
      <polygon points="256,161 303,140 256,119 209,140" fill={`url(#top4-${id})`} opacity="0.95" />
      <polyline points="209,140 256,119 303,140" fill="none" stroke="#f0abfc" strokeWidth="2" opacity="0.8" />

      {/* Connector lines */}
      <line x1="156" y1="287" x2="181" y2="238" stroke="#7c3aed" strokeWidth="1" opacity="0.4" />
      <line x1="181" y1="238" x2="196" y2="189" stroke="#a855f7" strokeWidth="1" opacity="0.4" />
      <line x1="196" y1="189" x2="209" y2="140" stroke="#c084fc" strokeWidth="1" opacity="0.4" />
      <line x1="356" y1="287" x2="331" y2="238" stroke="#7c3aed" strokeWidth="1" opacity="0.4" />
      <line x1="331" y1="238" x2="316" y2="189" stroke="#a855f7" strokeWidth="1" opacity="0.4" />
      <line x1="316" y1="189" x2="303" y2="140" stroke="#c084fc" strokeWidth="1" opacity="0.4" />

      {/* Node dots */}
      <circle cx="156" cy="287" r="4" fill="#7c3aed" filter={`url(#glow-${id})`} opacity="0.9" />
      <circle cx="356" cy="287" r="4" fill="#7c3aed" filter={`url(#glow-${id})`} opacity="0.9" />
      <circle cx="256" cy="257" r="4" fill="#8b5cf6" filter={`url(#glow-${id})`} opacity="0.9" />
      <circle cx="181" cy="238" r="3.5" fill="#a855f7" filter={`url(#glow-${id})`} opacity="0.9" />
      <circle cx="331" cy="238" r="3.5" fill="#a855f7" filter={`url(#glow-${id})`} opacity="0.9" />
      <circle cx="256" cy="211" r="3.5" fill="#c084fc" filter={`url(#glow-${id})`} opacity="0.9" />
      <circle cx="196" cy="189" r="3" fill="#c084fc" filter={`url(#glow-${id})`} opacity="0.9" />
      <circle cx="316" cy="189" r="3" fill="#c084fc" filter={`url(#glow-${id})`} opacity="0.9" />
      <circle cx="256" cy="166" r="3" fill="#d946ef" filter={`url(#glow-${id})`} opacity="0.9" />
      <circle cx="209" cy="140" r="3" fill="#e879f9" filter={`url(#glow-${id})`} />
      <circle cx="303" cy="140" r="3" fill="#e879f9" filter={`url(#glow-${id})`} />

      {/* Apex glow */}
      <circle cx="256" cy="119" r="14" fill="#d946ef" opacity="0.15" filter={`url(#apex-${id})`} />
      <circle cx="256" cy="119" r="7" fill="#f0abfc" opacity="0.7" filter={`url(#glow-${id})`} />
      <circle cx="256" cy="119" r="3" fill="#ffffff" opacity="0.95" />
    </svg>
  );
}

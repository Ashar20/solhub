export function SolanaMark({ className = "w-5 h-5" }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} fill="none" aria-hidden="true">
      <defs>
        <linearGradient id="solg" x1="0" y1="0" x2="24" y2="24">
          <stop offset="0" stopColor="#9945FF"/>
          <stop offset="1" stopColor="#14F195"/>
        </linearGradient>
      </defs>
      <path d="M4 7l4-3h12l-4 3H4zM4 13l4-3h12l-4 3H4zM4 19l4-3h12l-4 3H4z" fill="url(#solg)"/>
    </svg>
  );
}

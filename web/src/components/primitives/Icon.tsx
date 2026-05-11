import * as React from "react";
import { cn } from "@/lib/utils/cn";

const PATHS: Record<string, React.ReactNode> = {
  dashboard: <><rect x="3" y="3" width="7" height="9" rx="1.5"/><rect x="14" y="3" width="7" height="5" rx="1.5"/><rect x="14" y="12" width="7" height="9" rx="1.5"/><rect x="3" y="16" width="7" height="5" rx="1.5"/></>,
  workflows: <><circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="6" r="2.5"/><circle cx="12" cy="18" r="2.5"/><path d="M8 7l3 9M16 7l-3 9"/></>,
  builder: <><path d="M4 6h7M13 6h7M4 12h7M13 18h7M4 18h7M13 12h7"/><circle cx="11" cy="6" r="1.5"/><circle cx="13" cy="12" r="1.5"/><circle cx="11" cy="18" r="1.5"/></>,
  runs: <><path d="M5 4l4 4-4 4M9 8h7a4 4 0 014 4v0a4 4 0 01-4 4H5"/></>,
  marketplace: <><path d="M3 7l1.5-3h15L21 7M3 7v12a1 1 0 001 1h16a1 1 0 001-1V7M3 7h18M9 11a3 3 0 006 0"/></>,
  wallet: <><rect x="3" y="6" width="18" height="13" rx="2"/><path d="M3 9h15a3 3 0 013 3v0a3 3 0 01-3 3H3"/><circle cx="17" cy="12" r="1" fill="currentColor" stroke="none"/></>,
  versions: <><circle cx="6" cy="6" r="2"/><circle cx="6" cy="18" r="2"/><circle cx="18" cy="12" r="2"/><path d="M6 8v8M8 6h6a4 4 0 014 4M8 18h6a4 4 0 004-4"/></>,
  settings: <><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1"/></>,
  ai: <><path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3z"/><circle cx="18" cy="18" r="2"/></>,
  bell: <><path d="M6 8a6 6 0 0112 0c0 7 3 8 3 8H3s3-1 3-8M10 21a2 2 0 004 0"/></>,
  search: <><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/></>,
  plus: <path d="M12 5v14M5 12h14"/>,
  play: <path d="M7 4l13 8-13 8z" fill="currentColor" stroke="none"/>,
  pause: <><rect x="6" y="4" width="4" height="16" rx="1" fill="currentColor" stroke="none"/><rect x="14" y="4" width="4" height="16" rx="1" fill="currentColor" stroke="none"/></>,
  check: <path d="M5 12l4 4 10-10"/>,
  x: <path d="M5 5l14 14M19 5L5 19"/>,
  arrow: <path d="M5 12h14M13 6l6 6-6 6"/>,
  chevron: <path d="M9 6l6 6-6 6"/>,
  chevronDown: <path d="M6 9l6 6 6-6"/>,
  dot: <circle cx="12" cy="12" r="3" fill="currentColor" stroke="none"/>,
  bolt: <path d="M13 2L4 14h7l-1 8 9-12h-7l1-8z" fill="currentColor" stroke="none"/>,
  clock: <><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></>,
  eye: <><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/></>,
  code: <path d="M9 8l-5 4 5 4M15 8l5 4-5 4M14 4l-4 16"/>,
  db: <><ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6"/></>,
  flow: <><circle cx="6" cy="12" r="2.5"/><circle cx="18" cy="6" r="2.5"/><circle cx="18" cy="18" r="2.5"/><path d="M8 11l8-4M8 13l8 4"/></>,
  filter: <path d="M3 5h18l-7 9v6l-4-2v-4z"/>,
  git: <><circle cx="6" cy="6" r="2.5"/><circle cx="6" cy="18" r="2.5"/><circle cx="18" cy="12" r="2.5"/><path d="M6 8v8M8 6h6a4 4 0 014 4"/></>,
  spark: <path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M5.6 18.4l2.8-2.8M15.6 8.4l2.8-2.8"/>,
  upload: <path d="M12 17V3M5 10l7-7 7 7M3 21h18"/>,
  download: <path d="M12 3v14M5 14l7 7 7-7M3 21h18"/>,
  copy: <><rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V5a2 2 0 00-2-2H5a2 2 0 00-2 2v9a2 2 0 002 2h3"/></>,
  trash: <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13a2 2 0 002 2h6a2 2 0 002-2l1-13M9 7V4a1 1 0 011-1h4a1 1 0 011 1v3"/>,
  expand: <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>,
  drag: <><circle cx="9" cy="6" r="1" fill="currentColor" stroke="none"/><circle cx="9" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="9" cy="18" r="1" fill="currentColor" stroke="none"/><circle cx="15" cy="6" r="1" fill="currentColor" stroke="none"/><circle cx="15" cy="12" r="1" fill="currentColor" stroke="none"/><circle cx="15" cy="18" r="1" fill="currentColor" stroke="none"/></>,
  refresh: <path d="M3 12a9 9 0 0115-6.7L21 8M21 3v5h-5M21 12a9 9 0 01-15 6.7L3 16M3 21v-5h5"/>,
  sliders: <><path d="M4 6h10M18 6h2M4 12h2M10 12h10M4 18h12M20 18h-2"/><circle cx="16" cy="6" r="2"/><circle cx="8" cy="12" r="2"/><circle cx="18" cy="18" r="2"/></>,
  layers: <path d="M12 3l9 5-9 5-9-5 9-5zM3 13l9 5 9-5M3 18l9 5 9-5"/>,
  bug: <><rect x="8" y="6" width="8" height="14" rx="4"/><path d="M8 12H4M16 12h4M9 6c0-1.7 1.3-3 3-3s3 1.3 3 3M5 4l3 3M19 4l-3 3M5 20l3-3M19 20l-3-3"/></>,
  cloud: <path d="M7 18a5 5 0 010-10 6 6 0 0111 1 4 4 0 010 8z"/>,
  shield: <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6l8-3z"/>,
  key: <><circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3M14 9l3 3"/></>,
  logout: <path d="M15 3h3a2 2 0 012 2v14a2 2 0 01-2 2h-3M10 17l5-5-5-5M15 12H3"/>,
  bookmark: <path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/>,
  star: <path d="M12 2l3 7h7l-5.5 4 2 7-6.5-4-6.5 4 2-7L2 9h7z"/>,
  coins: <><circle cx="9" cy="9" r="6"/><path d="M22 13.6A6 6 0 1116.4 8M5.5 13.5l3 3"/></>,
  info: <><circle cx="12" cy="12" r="9"/><path d="M12 8h.01M11 12h1v4h1"/></>,
  warn: <path d="M12 3l10 18H2L12 3zM12 10v5M12 18h.01"/>,
  fire: <path d="M12 22c4 0 7-3 7-7 0-4-3-5-3-9 0 0-2 1-4 5-1-2-2-3-2-3s-5 4-5 9 3 5 7 5z"/>,
};

export type IconName = keyof typeof PATHS;

export interface IconProps extends Omit<React.SVGProps<SVGSVGElement>, "name" | "stroke"> {
  name: IconName;
  className?: string;
  stroke?: number;
}

export function Icon({ name, className, stroke = 1.6, ...rest }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={stroke}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={cn("w-4 h-4", className)}
      aria-hidden="true"
      {...rest}
    >
      {PATHS[name]}
    </svg>
  );
}

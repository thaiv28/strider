// Concentric topographic rings — ambient texture, never interactive.
export function Contours({ className }: { className?: string }) {
  const rings = [16, 30, 44, 58, 72, 86, 100];
  return (
    <svg
      viewBox="0 0 200 200"
      preserveAspectRatio="xMidYMid slice"
      className={className}
      fill="none"
      aria-hidden="true"
    >
      {rings.map((r, i) => (
        <ellipse
          key={r}
          cx={96 + i * 3}
          cy={108 - i * 2}
          rx={r}
          ry={r * 0.68}
          stroke="currentColor"
          strokeWidth={0.8}
          transform={`rotate(${-8 + i} 100 100)`}
        />
      ))}
    </svg>
  );
}

interface BadgeProps {
  children: React.ReactNode;
  color?: string;
  bgColor?: string;
  className?: string;
}

export function Badge({
  children,
  color = "text-text-secondary",
  bgColor = "bg-bg-card",
  className = "",
}: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-medium ${color} ${bgColor} ${className}`}
    >
      {children}
    </span>
  );
}

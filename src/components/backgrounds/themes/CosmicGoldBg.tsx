import { memo } from "react";

// Generates random stars for a space effect
const StarField = ({
  count = 50,
  color = "white",
}: {
  count?: number;
  color?: string;
}) => {
  const stars = Array.from({ length: count }).map((_, i) => ({
    id: i,
    top: `${Math.random() * 100}%`,
    left: `${Math.random() * 100}%`,
    size: Math.random() * 2 + 1 + "px",
    delay: Math.random() * 5 + "s",
    duration: Math.random() * 3 + 2 + "s",
  }));

  return (
    <div className="absolute inset-0">
      {stars.map((star) => (
        <div
          key={star.id}
          className="absolute rounded-full opacity-60 animate-pulse"
          style={{
            top: star.top,
            left: star.left,
            width: star.size,
            height: star.size,
            backgroundColor: color,
            animationDelay: star.delay,
            animationDuration: star.duration,
          }}
        />
      ))}
    </div>
  );
};

export const CosmicGoldBg = memo(() => {
  return (
    <>
      {/* Deep Cosmic Gradient */}
      <div className="absolute inset-0 bg-gradient-to-b from-transparent via-transparent to-[oklch(0.05_0.02_var(--color-gold-hue,50)_/_0.3)]" />

      {/* Stars Layer 1 - Tiny White Stars */}
      <StarField count={40} color="white" />

      {/* Stars Layer 2 - Gold Premium Dust */}
      <StarField count={20} color="var(--primary)" />

      {/* Nebula Glow */}
      <div
        className="absolute top-[20%] right-[30%] w-[40vw] h-[40vw] rounded-full blur-[150px] opacity-10"
        style={{
          background:
            "radial-gradient(circle, var(--primary) 0%, transparent 70%)",
        }}
      />
    </>
  );
});

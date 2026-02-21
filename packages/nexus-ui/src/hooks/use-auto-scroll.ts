import { useCallback, useEffect, useRef, useState } from "react";

const THRESHOLD = 50;

export function useAutoScroll() {
  const containerRef = useRef<HTMLDivElement>(null);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const [isAtBottom, setIsAtBottom] = useState(true);
  const isFollowingRef = useRef(true);

  // Synchronous scroll listener — updates both the ref (for instant decisions)
  // and the state (for UI like the scroll-to-bottom button).
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const handleScroll = () => {
      const nearBottom =
        el.scrollHeight - el.scrollTop - el.clientHeight < THRESHOLD;
      isFollowingRef.current = nearBottom;
      setIsAtBottom(nearBottom);
    };

    el.addEventListener("scroll", handleScroll, { passive: true });
    return () => el.removeEventListener("scroll", handleScroll);
  }, []);

  const scrollToBottom = useCallback(() => {
    const el = containerRef.current;
    if (el) el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  }, []);

  // Uses the ref (not state) so it always reflects the scroll position
  // from the most recent scroll event — no async observer lag.
  const scrollToBottomIfNeeded = useCallback(() => {
    if (isFollowingRef.current) {
      const el = containerRef.current;
      if (el) el.scrollTop = el.scrollHeight;
    }
  }, []);

  return {
    containerRef,
    sentinelRef,
    isAtBottom,
    scrollToBottom,
    scrollToBottomIfNeeded,
  };
}

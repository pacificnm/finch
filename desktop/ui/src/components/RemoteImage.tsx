import { useEffect, useState, type ReactNode } from "react";
import { fetchImage } from "../lib/nest";

export type RemoteImageProps = {
  url: string;
  alt: string;
  className?: string;
  tags?: string[];
  placeholder?: ReactNode;
};

export function RemoteImage({
  url,
  alt,
  className,
  tags,
  placeholder,
}: RemoteImageProps) {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setFailed(false);
    setSrc(null);

    fetchImage(url, tags)
      .then((response) => {
        if (!cancelled) {
          setSrc(`data:${response.mime};base64,${response.bytes_base64}`);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFailed(true);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [url, tags?.join(",")]);

  if (failed) {
    return (
      <div
        className={`flex items-center justify-center bg-nest-muted text-nest-foreground ${className ?? ""}`}
        role="img"
        aria-label={alt}
      >
        {placeholder ?? <span className="text-sm opacity-70">?</span>}
      </div>
    );
  }

  if (!src) {
    return (
      <div
        className={`animate-pulse bg-nest-muted ${className ?? ""}`}
        aria-hidden
      />
    );
  }

  return <img src={src} alt={alt} className={className} />;
}

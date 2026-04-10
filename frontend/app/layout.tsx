import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "CLNKR — Transcript Viewer",
  description: "Speech-to-text transcript viewer with word-level annotations",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="h-full">
      <body className="min-h-full flex flex-col">{children}</body>
    </html>
  );
}

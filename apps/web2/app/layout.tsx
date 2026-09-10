import type { Metadata } from "next";
import "./globals.css";
import { Providers } from "./providers";

export const metadata: Metadata = {
  title: "Reel — Your unified media catalogue",
  description: "Browse movies and series across your connected media services.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en">
      <body>
        <a
          href="#main-content"
          className="fixed top-3 left-3 z-50 -translate-y-24 rounded-lg bg-accent px-4 py-3 font-semibold text-background focus:translate-y-0"
        >
          Skip to content
        </a>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}

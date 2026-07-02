import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'Ferrum — Infrastructure Dashboard',
  description: 'Cyber-Industrial IaC control plane by Roberto de Souza',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="grid-bg min-h-screen antialiased">{children}</body>
    </html>
  );
}

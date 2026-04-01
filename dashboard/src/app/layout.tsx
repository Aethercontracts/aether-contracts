import type { Metadata } from 'next'
import { Inter } from 'next/font/google'
import './globals.css'

const inter = Inter({ subsets: ['latin'] })

export const metadata: Metadata = {
  title: 'AetherContracts | Trustless Creator Escrow',
  description: 'Formally Verified AI & Blockchain for the Creator Economy',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body className={`${inter.className} bg-[#0A0A0B] text-slate-200 min-h-screen antialiased`}>
        {children}
      </body>
    </html>
  )
}

export default function AuthLayout({ children }: { children: React.ReactNode }) {
  return (
    <main className="min-h-screen grid place-items-center bg-ink-50 grid-bg">
      {children}
    </main>
  );
}

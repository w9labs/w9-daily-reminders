import PreviewShell from '../components/PreviewShell'

export default function PreviewPage() {
  return (
    <>
      <header className="header">
        <h1>Preview · Daily Email</h1>
        <p>Regenerate the HTML payload exactly as the backend will send.</p>
      </header>
      <PreviewShell />
    </>
  )
}

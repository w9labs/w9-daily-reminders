import type { ReminderPreview } from '../../lib/types'

interface Props {
  preview?: ReminderPreview
}

export default function EmailPreview({ preview }: Props) {
  if (!preview) {
    return (
      <div className="preview-card">
        <h3>Preview</h3>
        <p>Configure settings and run Save + Preview to generate AI output.</p>
      </div>
    )
  }

  return (
    <div className="preview-card">
      <h3>{preview.subject}</h3>
      <p>Language · {preview.generatedLanguage}</p>
      {preview.weatherAdvisory && <p>{preview.weatherAdvisory}</p>}
      {preview.imageUrl && (
        <p>
          Image prompt satisfied via Pollinations: <a href={preview.imageUrl}>{preview.imageUrl}</a>
        </p>
      )}
      <div className="email-html" dangerouslySetInnerHTML={{ __html: preview.html }} />
    </div>
  )
}

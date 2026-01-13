/**
 * Utilities for handling YAML frontmatter in markdown content
 *
 * Frontmatter is managed internally by the app and should not be
 * displayed in the editor. Tags are edited via the UI only.
 */

/**
 * Strip YAML frontmatter from content, returning just the body
 * Frontmatter is delimited by --- at the start of the file
 */
export function stripFrontmatter(content: string): string {
  if (!content.startsWith('---')) {
    return content
  }

  const rest = content.slice(3)
  const endIndex = rest.indexOf('\n---')

  if (endIndex === -1) {
    return content
  }

  // Return everything after the closing --- and trim leading newlines
  return rest.slice(endIndex + 4).trimStart()
}

/**
 * Check if content has frontmatter
 */
export function hasFrontmatter(content: string): boolean {
  if (!content.startsWith('---')) {
    return false
  }
  return content.slice(3).includes('\n---')
}

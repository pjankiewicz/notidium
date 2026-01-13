import { Routes, Route } from 'react-router-dom'
import { Layout } from './components/layout/Layout'
import { ErrorBoundary } from './components/ui/ErrorBoundary'
import { HomePage } from './pages/HomePage'
import { NotesPage } from './pages/NotesPage'
import { NotePage } from './pages/NotePage'
import { SearchPage } from './pages/SearchPage'
import { TagsPage } from './pages/TagsPage'
import { StatsPage } from './pages/StatsPage'

function App() {
  return (
    <ErrorBoundary>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<HomePage />} />
          <Route path="notes" element={<NotesPage />} />
          <Route path="notes/:noteId" element={<NotePage />} />
          <Route path="search" element={<SearchPage />} />
          <Route path="tags" element={<TagsPage />} />
          <Route path="tags/:tag" element={<NotesPage />} />
          <Route path="stats" element={<StatsPage />} />
        </Route>
      </Routes>
    </ErrorBoundary>
  )
}

export default App

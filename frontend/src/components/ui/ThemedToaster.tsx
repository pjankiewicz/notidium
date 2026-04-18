import { Toaster } from 'sonner'
import { useSettingsStore } from '../../stores/settingsStore'

export function ThemedToaster() {
  const theme = useSettingsStore((state) => state.theme)
  return <Toaster position="bottom-right" theme={theme} />
}

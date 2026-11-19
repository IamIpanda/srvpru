import { Component, type ComponentChildren, type ComponentType } from 'preact'
import { Suspense, lazy } from 'preact/compat'
import { useMemo } from 'preact/hooks'
import { Typography } from 'antd'

type SlotBoundaryProps = {
  children: ComponentChildren
}

type SlotBoundaryState = {
  failure: string | null
}

type SlotComponent = ComponentType<{ enabled?: boolean }>

async function load(plugin: string, name: string): Promise<{ default: SlotComponent }> {
  const module = await import(/* @vite-ignore */ `/${plugin}.js`) as Record<string, SlotComponent>
  const component = module[name]
  if (component === undefined) throw new Error(`${plugin} 没有导出 ${name}`)
  return { default: component }
}

class SlotBoundary extends Component<SlotBoundaryProps, SlotBoundaryState> {
  state: SlotBoundaryState = { failure: null }

  componentDidCatch(failure: Error) {
    this.setState({ failure: failure.message })
  }

  render() {
    if (this.state.failure !== null) return <Typography.Text type="danger">{this.state.failure}</Typography.Text>
    return this.props.children
  }
}

export function Slot({ plugin, name, enabled }: { plugin: string; name: string; enabled?: boolean }) {
  const Loaded = useMemo(() => lazy(() => load(plugin, name)), [plugin, name])

  return (
    <SlotBoundary>
      <Suspense fallback={null}>
        <Loaded enabled={enabled} />
      </Suspense>
    </SlotBoundary>
  )
}

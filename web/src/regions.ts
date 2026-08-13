import raw from './regions.json'

export type RegionOption = { id: string; label: string }

export const REGIONS: RegionOption[] = raw.regions.map((region) => ({
  id: region.id,
  label: region.label,
}))

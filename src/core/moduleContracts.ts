export interface AppModuleDescriptor {
  id: string;
  title: string;
  layer: 'ui' | 'state' | 'service' | 'storage' | 'integration';
  status: 'stub' | 'active' | 'planned';
  dependsOn: string[];
}

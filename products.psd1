@{
  Products = @(
    @{
      Id = 'launcher'
      DisplayName = 'NEXORA'
      Directory = 'apps/nexora-launcher'
      Scripts = @{
        Dev = 'scripts/dev.ps1'
        Test = 'scripts/test.ps1'
        Debug = 'scripts/build-debug.ps1'
        Package = 'scripts/build-release.ps1'
      }
    },
    @{
      Id = 'isle'
      DisplayName = 'NEXORA ISLE · 岛屿'
      Directory = 'apps/nexora-isle'
      Scripts = @{
        Dev = 'scripts/dev.ps1'
        Test = 'scripts/test.ps1'
        Debug = 'scripts/build-debug.ps1'
        Package = 'scripts/package.ps1'
      }
    }
  )
}

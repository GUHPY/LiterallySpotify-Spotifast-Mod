# LiterallySpotify 0.1.1

The Windows app now defaults to its integrated title bar, keeping navigation, the profile and minimize/maximize/close controls on the same row.

## Fixed

- **Integrated Windows title bar by default.** New profiles and older settings that omit the title-bar preference use the customized layout. Explicit saved choices still work. If an existing profile has already saved the native frame, enable **Settings > Appearance > Custom title bar** once to restore the integrated layout.
- **Protected appearance preferences during engine maintenance.** The engine-update boundary now fingerprints the settings file and bundled themes, and the regression gate includes the Windows default, saved-choice handling, main-window recreation and native-decoration policy.
- **Complete theme test fixture.** The OLED Green palette required by the existing theme tests is included in the repository, so a clean checkout can compile the same checks as a local build.

All 42 declared UI/update regressions and seven maintenance tests passed locally before the Windows production build. The installer and portable ZIP use the fork's signed update stream.

## Thanks

Thanks to the Spotifast contributors for the native client foundation and to the users reporting layout regressions.

[Full changelog](https://github.com/GUHPY/LiterallySpotify-Spotifast-Mod/compare/v0.1.0...v0.1.1)

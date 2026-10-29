program GraphPalette;
{ Graph's palettes, GraphDefaults, the mode queries, SetAspectRatio,
  SetUserCharSize, the font and buffer fakes, and the two pages (the tests run with
  PASCRUST_GRAPH=headless). Self-checking: Free Pascal's Graph unit needs
  a real screen, so this .out is built from the checks, by hand. }
uses Graph;
var
  checks, failures: Integer;
  driver, mode, lo, hi, i, font: Integer;
  pal: PaletteType;
  xasp, yasp: Word;
  fills: FillSettingsType;

procedure Check(ok: Boolean; what: string);
begin
  checks := checks + 1;
  if ok then
    WriteLn('ok: ', what)
  else
  begin
    failures := failures + 1;
    WriteLn('FAILED: ', what);
  end;
end;

begin
  checks := 0;
  failures := 0;
  driver := Detect;
  InitGraph(driver, mode, '');
  GetPalette(pal);
  Check((pal.Size = 16) and (pal.Colors[Red] = 4) and (pal.Colors[Brown] = 20), 'GetPalette: the EGA colours');
  Check(GetPaletteSize = 16, 'GetPaletteSize');
  SetPalette(Red, 63);
  GetPalette(pal);
  Check(pal.Colors[Red] = 63, 'SetPalette shows in GetPalette');
  for i := 0 to MaxColors do
    pal.Colors[i] := -1;
  pal.Colors[Blue] := 5;
  SetAllPalette(pal);
  GetPalette(pal);
  Check((pal.Colors[Blue] = 5) and (pal.Colors[Red] = 63), 'SetAllPalette: -1 leaves a colour alone');
  SetPalette(16, 5);
  SetPalette(Green, 100);
  GetPalette(pal);
  Check((pal.Colors[Black] = 0) and (pal.Colors[Green] = 2), 'SetPalette out of range changes nothing');
  GetDefaultPalette(pal);
  Check((pal.Colors[Blue] = 1) and (pal.Colors[Red] = 4), 'GetDefaultPalette');
  SetRGBPalette(pal.Colors[Red], 63, 0, 0);
  SetColor(Red);
  PutPixel(5, 5, Yellow);
  GraphDefaults;
  GetPalette(pal);
  Check((GetColor = White) and (pal.Colors[Red] = 4), 'GraphDefaults resets colours and the palette');
  Check(GetPixel(5, 5) = Yellow, 'but not the picture');
  SetFillStyle(HatchFill, Red);
  SetFillStyle(13, Blue);
  SetFillStyle(UserFill, Blue);
  GetFillSettings(fills);
  Check((fills.Pattern = HatchFill) and (fills.Color = Red), 'SetFillStyle out of range changes nothing');
  SetAspectRatio(5000, 10000);
  GetAspectRatio(xasp, yasp);
  Check((xasp = 5000) and (yasp = 10000), 'SetAspectRatio and GetAspectRatio');
  Check(GetDriverName = 'EGAVGA', 'GetDriverName');
  Check((GetMaxMode = VGAHi) and (GetModeName(VGAHi) = '640 x 480 VGA'), 'GetMaxMode and GetModeName');
  GetModeRange(VGA, lo, hi);
  Check((lo = VGALo) and (hi = VGAHi), 'GetModeRange of VGA');
  GetModeRange(CGA, lo, hi);
  Check((lo = 0) and (hi = 4), 'GetModeRange of CGA');
  GetModeRange(99, lo, hi);
  Check((lo = -1) and (hi = -1), 'GetModeRange of no driver');
  font := InstallUserFont('LITT.CHR');
  SetGraphBufSize(8192);
  Check(font > GothicFont, 'InstallUserFont gives a new font number');
  SetTextStyle(DefaultFont, HorizDir, UserCharSize);
  SetUserCharSize(1, 1, 3, 1);
  Check(TextHeight('A') = 24, 'SetUserCharSize scales the text');
  SetActivePage(1);
  SetVisualPage(0);
  PutPixel(1, 1, Red);
  Check(GetPixel(1, 1) = Red, 'drawing on page 1');
  SetActivePage(0);
  Check(GetPixel(1, 1) = Black, 'leaves page 0 alone');
  SetActivePage(1);
  Check(GetPixel(1, 1) = Red, 'and page 1 keeps it');
  CloseGraph;
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.

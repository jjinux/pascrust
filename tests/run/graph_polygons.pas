program GraphPolygons;
{ DrawPoly and FillPoly with arrays of PointType, GetArcCoords, and the
  Get*Settings records, checked with GetPixel (the tests run with
  PASCRUST_GRAPH=headless). Self-checking: Free Pascal's Graph unit needs
  a real screen, so this .out is built from the checks, by hand. }
uses Graph;
type
  Triangle = array[1..3] of PointType;
var
  checks, failures: Integer;
  driver, mode: Integer;
  tri, path: Triangle;
  arcs: ArcCoordsType;
  lines: LineSettingsType;
  fills: FillSettingsType;
  texts: TextSettingsType;
  view: ViewPortType;
  pattern: FillPatternType;

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

procedure SetPoint(var p: PointType; x, y: Integer);
begin
  p.X := x;
  p.Y := y;
end;

{ The points arrive as a var parameter. }
procedure Outline(var points: Triangle; count: Integer);
begin
  DrawPoly(count, points);
end;

begin
  checks := 0;
  failures := 0;
  driver := Detect;
  InitGraph(driver, mode, '');
  SetPoint(tri[1], 10, 10);
  SetPoint(tri[2], 50, 10);
  SetPoint(tri[3], 30, 40);
  SetColor(White);
  SetFillStyle(SolidFill, Blue);
  FillPoly(3, tri);
  Check(GetPixel(30, 20) = Blue, 'FillPoly fills the inside');
  Check((GetPixel(10, 10) = White) and (GetPixel(30, 40) = White), 'and outlines the corners');
  Check(GetPixel(60, 20) = Black, 'and nothing outside');
  SetPoint(path[1], 100, 100);
  SetPoint(path[2], 140, 100);
  SetPoint(path[3], 140, 140);
  SetColor(Red);
  DrawPoly(3, path);
  Check((GetPixel(120, 100) = Red) and (GetPixel(140, 120) = Red), 'DrawPoly joins the points');
  Check(GetPixel(120, 120) = Black, 'without closing the shape');
  SetColor(Green);
  Outline(path, 2);
  Check((GetPixel(120, 100) = Green) and (GetPixel(140, 120) = Red), 'only NumPoints points, through a var parameter');
  Arc(200, 200, 0, 90, 50);
  GetArcCoords(arcs);
  Check((arcs.X = 200) and (arcs.Y = 200), 'GetArcCoords: the centre');
  Check((arcs.XStart = 250) and (arcs.YStart = 200) and (arcs.XEnd = 200) and (arcs.YEnd = 150), 'and the ends');
  SetLineStyle(UserBitLn, $F0F0, ThickWidth);
  GetLineSettings(lines);
  Check((lines.LineStyle = UserBitLn) and (lines.Pattern = $F0F0) and (lines.Thickness = ThickWidth), 'GetLineSettings');
  SetFillStyle(HatchFill, Red);
  GetFillSettings(fills);
  Check((fills.Pattern = HatchFill) and (fills.Color = Red), 'GetFillSettings');
  GetFillPattern(pattern);
  SetFillPattern(pattern, Cyan);
  GetFillSettings(fills);
  Check((fills.Pattern = UserFill) and (fills.Color = Cyan), 'GetFillSettings after SetFillPattern');
  SetTextStyle(SansSerifFont, VertDir, 4);
  SetTextJustify(CenterText, TopText);
  GetTextSettings(texts);
  Check((texts.Font = SansSerifFont) and (texts.Direction = VertDir) and (texts.CharSize = 4), 'GetTextSettings: font');
  Check((texts.Horiz = CenterText) and (texts.Vert = TopText), 'and justification');
  SetViewPort(10, 20, 300, 200, ClipOn);
  GetViewSettings(view);
  Check((view.x1 = 10) and (view.y1 = 20) and (view.x2 = 300) and (view.y2 = 200) and view.Clip, 'GetViewSettings');
  CloseGraph;
  WriteLn(checks - failures, ' of ', checks, ' checks passed');
  if failures > 0 then
    Halt(1);
end.

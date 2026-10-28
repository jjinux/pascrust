program GraphBasics;
{ The Graph unit, drawn without a window (the tests run with
  PASCRUST_GRAPH=headless) and checked with GetPixel. The expected output is
  written by hand: Free Pascal's Graph unit needs a real screen. }
uses Graph;
var
   driver, mode, i: integer;
   pattern: FillPatternType;
begin
   driver := Detect;
   InitGraph(driver, mode, 'C:\TP\BGI');
   writeln('driver ', driver, ', mode ', mode, ', result ', GraphResult);
   writeln('screen ', GetMaxX, ' x ', GetMaxY, ', colours 0..', GetMaxColor);

   SetColor(Red);
   Line(10, 10, 20, 10);
   writeln('line: ', GetPixel(10, 10), ' ', GetPixel(20, 10), ' ', GetPixel(21, 10));
   SetLineStyle(SolidLn, 0, ThickWidth);
   Line(10, 30, 50, 30);
   writeln('thick: ', GetPixel(30, 29), ' ', GetPixel(30, 31), ' ', GetPixel(30, 32));
   SetLineStyle(DottedLn, 0, NormWidth);
   Line(0, 50, 7, 50);
   write('dotted: ');
   for i := 0 to 7 do
      write(GetPixel(i, 50));
   writeln;

   SetFillStyle(SolidFill, Blue);
   Bar(100, 100, 109, 109);
   writeln('bar: ', GetPixel(100, 100), ' ', GetPixel(109, 109), ' ', GetPixel(110, 110));
   SetColor(Green);
   Circle(200, 200, 20);
   writeln('circle: ', GetPixel(220, 200), ' ', GetPixel(200, 180), ' ', GetPixel(200, 200));
   SetFillStyle(SolidFill, Yellow);
   FloodFill(200, 200, Green);
   writeln('flood: ', GetPixel(200, 200), ' ', GetPixel(230, 200));
   GetFillPattern(pattern);
   writeln('pattern: ', pattern[1], ' ', pattern[8]);

   SetViewPort(300, 300, 400, 400, ClipOn);
   PutPixel(0, 0, White);
   PutPixel(200, 0, White);
   SetViewPort(0, 0, GetMaxX, GetMaxY, ClipOn);
   writeln('viewport: ', GetPixel(300, 300), ' ', GetPixel(500, 300));

   SetTextJustify(CenterText, CenterText);
   writeln('text: ', TextWidth('Hello'), ' x ', TextHeight('Hello'));
   MoveTo(5, 5);
   LineTo(15, 5);
   writeln('position: ', GetX, ', ', GetY);
   SetBkColor(Blue);
   writeln('background: ', GetBkColor, ', pixel ', GetPixel(0, 479));
   CloseGraph;
   writeln('done');
end.

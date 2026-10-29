program GraphImages;
{ GetImage, PutImage with its five modes, and ImageSize, with the image
  in TP's own buffer layout: two Words (width - 1, height - 1), then each
  row as bit planes 3 down to 0 (and 2 bytes more that ImageSize counts
  but GetImage leaves alone). The buffer comes
  from GetMem, a static array, or a file. Self-checking: Free Pascal's
  Graph unit needs a real screen (and keeps images its own way), so this
  .out is built from the checks, by hand. }
uses Graph;
type
   Header = record
      width, height: Word;
   end;
var
   checks, failures: Integer;
   driver, mode: Integer;
   p: Pointer;
   size: Word;
   buf: array[1..86] of Byte;
   head: Header;
   f: file;

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
   Check(ImageSize(0, 0, 15, 15) = 134, 'ImageSize: 6 + 16 rows of 4 planes of 2 bytes');
   Check(ImageSize(0, 0, 9, 9) = 86, 'ImageSize rounds a row up to 8 pixels');
   Check((GraphResult = grOk) and (ImageSize(0, 0, 639, 479) = 0), 'ImageSize of 64K or more is 0');
   Check(GraphResult = grError, 'with GraphResult grError');
   Check(GraphResult = grOk, 'reading GraphResult resets it');
   PutPixel(0, 0, LightBlue);
   PutPixel(9, 9, Brown);
   size := ImageSize(0, 0, 9, 9);
   GetMem(p, size);
   GetImage(0, 0, 9, 9, p^);
   Move(p^, head, SizeOf(head));
   Check((head.width = 9) and (head.height = 9), 'the header: width - 1, height - 1');
   PutImage(100, 50, p^, NormalPut);
   Check((GetPixel(100, 50) = LightBlue) and (GetPixel(109, 59) = Brown), 'PutImage NormalPut');
   PutImage(100, 50, p^, XORPut);
   Check((GetPixel(100, 50) = Black) and (GetPixel(109, 59) = Black), 'XORPut twice erases');
   PutPixel(100, 50, Red);
   PutImage(100, 50, p^, OrPut);
   Check(GetPixel(100, 50) = Red or LightBlue, 'OrPut');
   PutImage(100, 50, p^, AndPut);
   Check(GetPixel(100, 50) = (Red or LightBlue) and LightBlue, 'AndPut');
   PutImage(100, 50, p^, NotPut);
   Check((GetPixel(100, 50) = 15 - LightBlue) and (GetPixel(101, 50) = White), 'NotPut');
   { The planes: LightBlue is 1001, so pixel 0 is in planes 3 and 0. }
   GetImage(0, 0, 9, 9, buf);
   Check((buf[5] = $80) and (buf[7] = 0) and (buf[9] = 0) and (buf[11] = $80), 'row 0: planes 3 down to 0');
   { Brown is 0110: the last row, planes 2 and 1, pixel 9 is bit 6 of byte 2. }
   Check((buf[77] = 0) and (buf[80] = $40) and (buf[82] = $40) and (buf[84] = 0), 'the last row');
   { A sprite saved to a file and drawn from it. }
   Assign(f, 'sprite.img');
   Rewrite(f, 1);
   BlockWrite(f, p^, size);
   Close(f);
   FillChar(buf, SizeOf(buf), 0);
   Reset(f, 1);
   BlockRead(f, buf, FileSize(f));
   Close(f);
   Erase(f);
   ClearDevice;
   PutImage(200, 100, buf, NormalPut);
   Check((GetPixel(200, 100) = LightBlue) and (GetPixel(209, 109) = Brown), 'an image from a file');
   { The viewport moves the image, and clips it if it's set to. }
   SetViewPort(300, 300, 302, 302, ClipOn);
   PutImage(0, 0, buf, NormalPut);
   SetViewPort(0, 0, GetMaxX, GetMaxY, ClipOn);
   Check((GetPixel(300, 300) = LightBlue) and (GetPixel(309, 309) = Black), 'PutImage clips to the viewport');
   SetViewPort(400, 400, 402, 402, ClipOff);
   PutImage(0, 0, buf, NormalPut);
   SetViewPort(0, 0, GetMaxX, GetMaxY, ClipOn);
   Check(GetPixel(409, 409) = Brown, 'but not with ClipOff');
   FreeMem(p, size);
   CloseGraph;
   WriteLn(checks - failures, ' of ', checks, ' checks passed');
   if failures > 0 then
      Halt(1);
end.

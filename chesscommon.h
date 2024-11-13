#ifndef CHESS_COMMON_H
#define CHESS_COMMON_H

#include <GL/gl.h>
#include <iostream>
#include "uicommon.h"
#include <bitset>
#include <unordered_map>
enum Parity {
  WHITE   = 0b00000000,
  BLACK   = 0b00001000,
};
enum Pieces {
  ROOK    = 0b00000001,
  KNIGHT  = 0b00000010,
  BISHOP  = 0b00000011,
  QUEEN   = 0b00000100,
  KING    = 0b00000101,
  PAWN    = 0b00000110,
  NONE    = 0b00000111,
};
enum FENs {
  FEN_START     = 0,
  FEN_ENPASSANT = 1,
};
static const std::string FEN_EXAMPLES[] = {
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 
  "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
};
static const Pieces PIECENAMES[] = {ROOK, KNIGHT, BISHOP, QUEEN, KING, PAWN, NONE};
static const char PIECECHARS[] = {'R', 'N', 'B', 'Q', 'K', 'P', '.'};
static const char toChar(const uint8_t &byte) { return PIECECHARS[(uint8_t) (byte & NONE) - 1]; }
namespace Helper {
  static const int algLetterToX(char c) {
    return ((int)c) - 97;
  }
};

struct Piece {
  Icon icon;
  uint8_t byte;
  Parity parity;
  Pieces piece;
  Point pos, winPos;
  bool hasMoved;
  std::vector<Point> threatening;
  std::vector<Point> moves;
  Piece(uint8_t sig, const Point &pos_, Icon icn) : byte(sig), pos(pos_), icon(icn){
    parity = ((byte >> 3) == 0b0000 ? WHITE : BLACK);
    piece = PIECENAMES[(int)(byte & NONE) - 1];
    hasMoved = false;
    updateWinPos();
  }
  inline void updateWinPos(){ winPos = Point(icon.w * pos.x, icon.h * pos.y); }
  void draw(){
    if(icon.fake) return;
    glBegin(GL_POINTS);
    Point cursor = winPos;
    double cReturn = cursor.x;
    for(int i = 0; i < icon.w * icon.h; i++){
      if(icon.img[i] == 0xff){
        glColor3f(icon.colors[i][0], icon.colors[i][1], icon.colors[i][2]);
        glVertex2f(cursor.x, cursor.y);
      }
      if(i % icon.w == 0){
        cursor.y += 1;
        cursor.x = cReturn;
      } else {
        cursor.x++;
      }
    }
  }
  const bool operator==(const Piece &o){
    if(o.byte == byte && o.pos == pos && o.winPos == winPos) return true;
    return false;
  }
  char toChar(){ return PIECECHARS[(uint8_t) (byte & NONE) - 1]; }
  void updateValidMoves(uint8_t (&tiles)[8][8], const Point &enpassant){
    if(piece == ROOK) rook(tiles);
    if(piece == PAWN) pawn(tiles, enpassant);
    if(piece == BISHOP) bishop(tiles);
    if(piece == KNIGHT) knight(tiles);
    if(piece == KING) king(tiles);
    if(piece == QUEEN){
      rook(tiles);
      bishop(tiles);
    }
  }
  void move(const Point &to){
    pos = to;
    winPos = Point(icon.w * pos.x, icon.h * pos.y);
    hasMoved = true;
  }
private:
  void rook(uint8_t (&tiles)[8][8]){
    bool hits[] = {false, false, false, false};
    for(int i = 1; i < 8; i++){
      Point ps[4] = {
        Point(pos.x + i, pos.y),
        Point(pos.x - i, pos.y),
        Point(pos.x, pos.y + i),
        Point(pos.x, pos.y - i)
      };
      for(int k = 0; k < 4; k++){
        if(!ps[k].isValid()) hits[k] = true;
        if(hits[k]) continue;
        if((tiles[ps[k].x][ps[k].y] >> 3) != (parity >> 3) && (tiles[ps[k].x][ps[k].y] != NONE)){
          hits[k] = true;
          threatening.push_back(ps[k]);
          moves.push_back(ps[k]);
        } else if(tiles[ps[k].x][ps[k].y] == NONE){
          moves.push_back(ps[k]);
        } else {
          hits[k] = true;
        }
      }
    }
  }
  void pawn(uint8_t (&tiles)[8][8], const Point &enpassant){
    int par = parity == WHITE ? 1 : -1;
    Point basic = Point(pos.x, pos.y + par);
    if(tiles[basic.x][basic.y] == NONE){
      moves.push_back(basic);
      if(!hasMoved){
        Point dbl = Point(basic.x, basic.y + par);
        if(tiles[dbl.x][dbl.y] == NONE) moves.push_back(dbl);
      }
    }
    Point d[2] = {Point(basic.x - 1, basic.y), Point(basic.x + 1, basic.y)};
    for(int i = 0; i < 2; i++){
      if(d[i].isValid()){
        if((tiles[d[i].x][d[i].y] >> 3) != (parity >> 3) && tiles[d[i].x][d[i].y] != NONE){
          moves.push_back(d[i]);
          threatening.push_back(d[i]);
        } else if(tiles[d[i].x][d[i].y] == NONE){
          if(enpassant == d[i]){
            moves.push_back(d[i]);
            threatening.push_back(d[i]);
          }
        }
      }
    }
  }
  void bishop(uint8_t (&tiles)[8][8]){
    bool hits[] = {false, false, false, false};
    for(int i = 1; i < 8; i++){
      Point ps[4] = {
        Point(pos.x + i, pos.y + i),
        Point(pos.x - i, pos.y + i),
        Point(pos.x - i, pos.y - i),
        Point(pos.x + i, pos.y - i)
      };
      for(int k = 0; k < 4; k++){
        if(!ps[k].isValid()) hits[k] = true;
        if(hits[k]) continue;
        if((tiles[ps[k].x][ps[k].y] >> 3) != (parity >> 3) && tiles[ps[k].x][ps[k].y] != NONE){
          hits[k] = true;
          moves.push_back(ps[k]);
          threatening.push_back(ps[k]);
        } else if(tiles[ps[k].x][ps[k].y] == NONE){
          moves.push_back(ps[k]);
        } else {
          hits[k] = true;
        }
      }
    }

  }
  void knight(uint8_t (&tiles)[8][8]){
    Point ps[8] = {
      Point(pos.x + 1, pos.y + 2),
      Point(pos.x + 1, pos.y - 2),
      Point(pos.x - 1, pos.y + 2),
      Point(pos.x - 1, pos.y - 2),
      Point(pos.x + 2, pos.y + 1),
      Point(pos.x + 2, pos.y - 1),
      Point(pos.x - 2, pos.y + 1),
      Point(pos.x - 2, pos.y - 1),
    };
    for(int i = 0; i < 8; i++){
      if(!ps[i].isValid()) continue;
      if(tiles[ps[i].x][ps[i].y] == NONE){
        moves.push_back(ps[i]);
      } else if((tiles[ps[i].x][ps[i].y] >> 3) != (parity >> 3)){
        threatening.push_back(ps[i]);
        moves.push_back(ps[i]);
      }
    }
  }
  void king(uint8_t (&tiles)[8][8]){
    for(int y = -1; y < 2; y++){
      for(int x = -1; x < 2; x++){
        if(x == 0 && y == 0) continue;
        Point ps = Point(pos.x + x, pos.y + y);
        if(!ps.isValid()) continue;
        if(tiles[ps.x][ps.y] == NONE){
          moves.push_back(ps);
        } else if((tiles[ps.x][ps.y] >> 3) != (parity >> 3)){
          moves.push_back(ps);
          threatening.push_back(ps);
        }
      }
    }
  }
};




#endif

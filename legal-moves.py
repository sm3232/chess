from typing import List, Tuple, Set
from dataclasses import dataclass
from enum import Enum

class Color(Enum):
    WHITE = 'white'
    BLACK = 'black'

class PieceType(Enum):
    PAWN = 'P'
    KNIGHT = 'N'
    BISHOP = 'B'
    ROOK = 'R'
    QUEEN = 'Q'
    KING = 'K'

@dataclass
class Piece:
    type: PieceType
    color: Color

class ChessBoard:
    def __init__(self, fen: str):
        self.board = [[None for _ in range(8)] for _ in range(8)]
        self.current_turn = None
        self.castling_rights = {'K': False, 'Q': False, 'k': False, 'q': False}
        self.en_passant_target = None
        self.halfmove_clock = 0
        self.fullmove_number = 1
        self._parse_fen(fen)

    def _parse_fen(self, fen: str) -> None:
        """Parse FEN string and set up the board state."""
        parts = fen.split()
        board_str = parts[0]
        
        # Parse piece positions
        row = 0
        col = 0
        for char in board_str:
            if char == '/':
                row += 1
                col = 0
            elif char.isdigit():
                col += int(char)
            else:
                color = Color.WHITE if char.isupper() else Color.BLACK
                piece_type = PieceType(char.upper())
                self.board[row][col] = Piece(piece_type, color)
                col += 1

        # Parse active color
        self.current_turn = Color.WHITE if parts[1] == 'w' else Color.BLACK

        # Parse castling rights
        self.castling_rights = {
            'K': 'K' in parts[2],
            'Q': 'Q' in parts[2],
            'k': 'k' in parts[2],
            'q': 'q' in parts[2]
        }

        # Parse en passant target square
        self.en_passant_target = None if parts[3] == '-' else self._algebraic_to_coords(parts[3])

        # Parse half and full move counts
        self.halfmove_clock = int(parts[4])
        self.fullmove_number = int(parts[5])

    def _algebraic_to_coords(self, algebraic: str) -> Tuple[int, int]:
        """Convert algebraic notation (e.g., 'e4') to board coordinates (row, col)."""
        col = ord(algebraic[0].lower()) - ord('a')
        row = 8 - int(algebraic[1])
        return (row, col)

    def _coords_to_algebraic(self, row: int, col: int) -> str:
        """Convert board coordinates to algebraic notation."""
        return f"{chr(col + ord('a'))}{8 - row}"

    def get_legal_moves(self) -> List[Tuple[str, str]]:
        """Returns all legal moves in the current position."""
        moves = []
        for row in range(8):
            for col in range(8):
                piece = self.board[row][col]
                if piece and piece.color == self.current_turn:
                    piece_moves = self._get_piece_moves(row, col)
                    moves.extend(piece_moves)
        return moves

    def _get_piece_moves(self, row: int, col: int) -> List[Tuple[str, str]]:
        """Get all legal moves for a piece at the given position."""
        piece = self.board[row][col]
        if not piece:
            return []

        moves = []
        if piece.type == PieceType.PAWN:
            moves.extend(self._get_pawn_moves(row, col))
        elif piece.type == PieceType.KNIGHT:
            moves.extend(self._get_knight_moves(row, col))
        elif piece.type == PieceType.BISHOP:
            moves.extend(self._get_bishop_moves(row, col))
        elif piece.type == PieceType.ROOK:
            moves.extend(self._get_rook_moves(row, col))
        elif piece.type == PieceType.QUEEN:
            moves.extend(self._get_queen_moves(row, col))
        elif piece.type == PieceType.KING:
            moves.extend(self._get_king_moves(row, col))

        # Filter out moves that would leave king in check
        legal_moves = []
        for move in moves:
            if not self._would_be_in_check_after_move(move[0], move[1]):
                legal_moves.append(move)

        return legal_moves

    def _get_pawn_moves(self, row: int, col: int) -> List[Tuple[str, str]]:
        """Get all possible pawn moves."""
        moves = []
        direction = -1 if self.board[row][col].color == Color.WHITE else 1
        start_row = 6 if self.board[row][col].color == Color.WHITE else 1

        # Forward moves
        if 0 <= row + direction < 8 and not self.board[row + direction][col]:
            moves.append((self._coords_to_algebraic(row, col), 
                         self._coords_to_algebraic(row + direction, col)))
            
            # Double move from starting position
            if row == start_row and not self.board[row + 2*direction][col]:
                moves.append((self._coords_to_algebraic(row, col),
                            self._coords_to_algebraic(row + 2*direction, col)))

        # Captures
        for col_offset in [-1, 1]:
            new_col = col + col_offset
            if 0 <= new_col < 8:
                # Normal capture
                if (0 <= row + direction < 8 and 
                    self.board[row + direction][new_col] and 
                    self.board[row + direction][new_col].color != self.board[row][col].color):
                    moves.append((self._coords_to_algebraic(row, col),
                                self._coords_to_algebraic(row + direction, new_col)))
                
                # En passant
                if self.en_passant_target and (row + direction, new_col) == self.en_passant_target:
                    moves.append((self._coords_to_algebraic(row, col),
                                self._coords_to_algebraic(row + direction, new_col)))

        return moves

    def _get_knight_moves(self, row: int, col: int) -> List[Tuple[str, str]]:
        """Get all possible knight moves."""
        moves = []
        offsets = [(-2, -1), (-2, 1), (-1, -2), (-1, 2),
                  (1, -2), (1, 2), (2, -1), (2, 1)]

        for row_offset, col_offset in offsets:
            new_row = row + row_offset
            new_col = col + col_offset
            if (0 <= new_row < 8 and 0 <= new_col < 8 and
                (not self.board[new_row][new_col] or 
                 self.board[new_row][new_col].color != self.board[row][col].color)):
                moves.append((self._coords_to_algebraic(row, col),
                            self._coords_to_algebraic(new_row, new_col)))

        return moves

    def _get_sliding_piece_moves(self, row: int, col: int, directions: List[Tuple[int, int]]) -> List[Tuple[str, str]]:
        """Get all possible moves for sliding pieces (bishop, rook, queen)."""
        moves = []
        for direction in directions:
            for i in range(1, 8):
                new_row = row + direction[0] * i
                new_col = col + direction[1] * i
                if not (0 <= new_row < 8 and 0 <= new_col < 8):
                    break
                if self.board[new_row][new_col]:
                    if self.board[new_row][new_col].color != self.board[row][col].color:
                        moves.append((self._coords_to_algebraic(row, col),
                                    self._coords_to_algebraic(new_row, new_col)))
                    break
                moves.append((self._coords_to_algebraic(row, col),
                            self._coords_to_algebraic(new_row, new_col)))
        return moves

    def _get_bishop_moves(self, row: int, col: int) -> List[Tuple[str, str]]:
        """Get all possible bishop moves."""
        directions = [(-1, -1), (-1, 1), (1, -1), (1, 1)]
        return self._get_sliding_piece_moves(row, col, directions)

    def _get_rook_moves(self, row: int, col: int) -> List[Tuple[str, str]]:
        """Get all possible rook moves."""
        directions = [(-1, 0), (1, 0), (0, -1), (0, 1)]
        return self._get_sliding_piece_moves(row, col, directions)

    def _get_queen_moves(self, row: int, col: int) -> List[Tuple[str, str]]:
        """Get all possible queen moves."""
        directions = [(-1, -1), (-1, 1), (1, -1), (1, 1),
                     (-1, 0), (1, 0), (0, -1), (0, 1)]
        return self._get_sliding_piece_moves(row, col, directions)

    def _get_king_moves(self, row: int, col: int) -> List[Tuple[str, str]]:
        """Get all possible king moves, including castling."""
        moves = []
        # Normal king moves
        for row_offset in [-1, 0, 1]:
            for col_offset in [-1, 0, 1]:
                if row_offset == 0 and col_offset == 0:
                    continue
                new_row = row + row_offset
                new_col = col + col_offset
                if (0 <= new_row < 8 and 0 <= new_col < 8 and
                    (not self.board[new_row][new_col] or 
                     self.board[new_row][new_col].color != self.board[row][col].color)):
                    moves.append((self._coords_to_algebraic(row, col),
                                self._coords_to_algebraic(new_row, new_col)))

        # Castling
        if not self._is_in_check():
            if self.current_turn == Color.WHITE:
                if self.castling_rights['K'] and self._can_castle_kingside():
                    moves.append((self._coords_to_algebraic(row, col),
                                self._coords_to_algebraic(row, col + 2)))
                if self.castling_rights['Q'] and self._can_castle_queenside():
                    moves.append((self._coords_to_algebraic(row, col),
                                self._coords_to_algebraic(row, col - 2)))
            else:
                if self.castling_rights['k'] and self._can_castle_kingside():
                    moves.append((self._coords_to_algebraic(row, col),
                                self._coords_to_algebraic(row, col + 2)))
                if self.castling_rights['q'] and self._can_castle_queenside():
                    moves.append((self._coords_to_algebraic(row, col),
                                self._coords_to_algebraic(row, col - 2)))

        return moves

    def _can_castle_kingside(self) -> bool:
        """Check if kingside castling is possible."""
        row = 7 if self.current_turn == Color.WHITE else 0
        return (not self.board[row][5] and 
                not self.board[row][6] and 
                not self._is_square_attacked(row, 5) and 
                not self._is_square_attacked(row, 6))

    def _can_castle_queenside(self) -> bool:
        """Check if queenside castling is possible."""
        row = 7 if self.current_turn == Color.WHITE else 0
        return (not self.board[row][1] and 
                not self.board[row][2] and 
                not self.board[row][3] and 
                not self._is_square_attacked(row, 2) and 
                not self._is_square_attacked(row, 3))

    def _is_square_attacked(self, row: int, col: int) -> bool:
        """Check if a square is attacked by any enemy piece."""
        original_turn = self.current_turn
        self.current_turn = Color.BLACK if self.current_turn == Color.WHITE else Color.WHITE
        
        for r in range(8):
            for c in range(8):
                piece = self.board[r][c]
                if piece and piece.color == self.current_turn:
                    moves = self._get_piece_moves(r, c)
                    for move in moves:
                        if self._algebraic_to_coords(move[1]) == (row, col):
                            self.current_turn = original_turn
                            return True
        
        self.current_turn = original_turn
        return False

    def _is_in_check(self) -> bool:
        """Check if the current player's king is in check."""
        king_pos = None
        for row in range(8):
            for col in range(8):
                piece = self.board[row][col]
                if (piece and piece.type == PieceType.KING and 
                    piece.color == self.current_turn):
                    king_pos = (row, col)
                    break
            if king_pos:
                break
        
        return self._is_square_attacked(*king_pos)

    def _would_be_in_check_after_move(self, from_sq: str, to_sq: str) -> bool:
        """Check if making a move would leave the king in check."""
        from_pos = self._algebraic_to_coords(from_sq)
        to_pos = self._algebraic_to_coords(to_sq)
        
        # Make move
        captured_piece = self.board[to_pos[0]][to_pos[1]]
        self.board[to_pos[0]][to_pos[1]] = self.board[from_pos[0]][from_pos[1]]
        self.board[from_pos[0]][from_pos[1]] = None
        
        # Check if in check
        in_check = self._is_in_check()
        
        # Undo move
        self.board[from_pos[0]][from_pos[1]] = self.board[to_pos[0]][to_pos[1]]
        self.board[to_pos[0]][to_pos[1]] = captured_piece
        
        return in_check

# Example usage
def main():
    # Starting position
    initial_fen = "rnbqkbnr

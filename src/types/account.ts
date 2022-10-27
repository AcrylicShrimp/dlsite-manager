export interface Account {
  id: number;
  username: string;
  password: string;
  memo?: string;
  created_at: number;
  updated_at: number;
}

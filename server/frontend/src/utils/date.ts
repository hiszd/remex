import moment from "moment";

export function formatDate(date: Date | string): string {
  const d = new Date(date);
  return moment(d).format('MM-DD-YY [at] hh:mm:ss A');
}

export function formatDistanceToNow(date: Date | string): string {
  const d = new Date(date);
  return moment(d).fromNow();
}

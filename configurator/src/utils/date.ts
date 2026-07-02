import moment from "moment";

export function formatDate(date: Date | string): string {
  const d = new Date(date);
  return moment(d).format('MM-DD-YY [at] hh:mm:ss A');
}

export function formatDistanceToNow(date: Date | string): string {
  const d = new Date(date);
  return moment(d).fromNow();
}

export function formatDuration(start: Date, end: Date): string {
  const ms = end.getTime() - start.getTime();
  if (ms < 0) return "-";
  const dur = moment.duration(ms);
  if (dur.asHours() >= 1) {
    return `${Math.floor(dur.asHours())}h ${dur.minutes()}m ${dur.seconds()}s`;
  }
  if (dur.asMinutes() >= 1) {
    return `${dur.minutes()}m ${dur.seconds()}s`;
  }
  if (dur.asSeconds() >= 1) {
    return `${dur.seconds()}s`;
  }
  return `<1s`;
}



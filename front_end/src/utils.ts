import { DependencyList, useEffect, useState } from "react";

type Args = RequestInit & { deps?: DependencyList };

export function useApi<T>(url: null): undefined;
export function useApi<T>(url: string): [T | undefined, any];
export function useApi<T>(url: string | null, args?: Args): [T | undefined, any] | undefined;
export function useApi<T>(url: string | null, args?: Args): [T | undefined, any] | undefined {
    const [value, setValue] = useState(undefined);
    const [error, setError] = useState(null);

    useEffect(() => {
        async function asyncFetch() {
            if (url === null) {
                return;
            }

            const response = await fetch(`/api${url}`, args);
            const data = await response.json();

            if (response.ok) {
                setValue(data);
            } else {
                setError(data);
            }
        }

        asyncFetch();
    }, args?.deps || []);

    if (url === null) return undefined;
    return [value, error];
}

export async function fetchApi(url: string, init?: RequestInit) {
    return fetch(`/api${url}`, init);
}

export function formatDate(date: Date): string {
    const day = date.getDate();

    if (Number.isNaN(day)) {
        console.error("invalid date to format")
        return "{invalid date}";
    }

    let suffix;
    switch (day % 10) {
        case 1:
            suffix = "st";
            break;
        case 2:
            suffix = "nd";
            break;
        case 3:
            suffix = "rd";
            break;
        default:
            suffix = "th";
    }

    if (day >= 11 && day <= 13) {
        suffix = "th";
    }

    const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const month = months[date.getMonth()];

    const year = date.getFullYear();

    return `${day}${suffix} ${month} ${year}`;
}


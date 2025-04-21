import { DependencyList, useEffect, useState } from "react";

type Args = RequestInit & { deps?: DependencyList };

type ApiError = { status: number } | undefined;

export function useApi<T>(url: null): undefined;
export function useApi<T>(url: string): [T | undefined, ApiError];
export function useApi<T>(url: string | null, args?: Args): [T | undefined, ApiError] | undefined;
export function useApi<T>(url: string | null, args?: Args): [T | undefined, ApiError] | undefined {
    const [value, setValue] = useState(undefined);
    const [error, setError] = useState<ApiError>(undefined);

    useEffect(() => {
        async function asyncFetch() {
            if (url === null) {
                return;
            }

            const response = await fetch(`/api${url}`, args);

            if (response.ok) {
                const data = await response.json();
                setValue(data);
            } else {
                setError({ status: response.status });
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


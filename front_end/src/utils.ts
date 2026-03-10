import { DependencyList, useEffect, useState } from "react";

type Args = RequestInit & { deps?: DependencyList };

export type ApiError = { status: number } | undefined;

// represents a state or value which is retrieved via HTTP request to the backend API
export function useApi<T>(url: null, args?: Args): undefined;
export function useApi<T>(url: string, args?: Args): [T | undefined, ApiError];
export function useApi<T>(url: string | null, args?: Args): [T | undefined, ApiError] | undefined;
export function useApi<T>(url: string | null, args?: Args): [T | undefined, ApiError] | undefined {
    // state representing the value from the API if it has been returned yet (`undefined` otherwise)
    const [value, setValue] = useState(undefined);
    // state representing whether an error has been returned by the API
    const [error, setError] = useState<ApiError>(undefined);

    useEffect(() => {
        async function asyncFetch() {
            if (url === null) {
                return;
            }

            // fetch from the API
            const response = await fetchApi(url, args);

            // if the request was a success, convert the returned data into an object from its JSON representation, and update the `value` state
            if (response.ok) {
                const data = await response.json();
                setValue(data);
            } else {
                // if it failed, update the `error` state, with the returned status code specified
                setError({ status: response.status });
            }
        }

        asyncFetch();
        
    // re-fetch from API whenever any of the specified dependencies are updated
    }, args?.deps || []);

    if (url === null) return undefined;
    return [value, error];
}

// make a HTTP request to the API at the given path
export async function fetchApi(url: string, init?: RequestInit) {
    return fetch(`/api${url}`, init);
}

// format dates in a user presentable format for showing on the web frontend
export function formatDate(date: Date): string {
    // get the day as a number
    const day = date.getDate();

    if (Number.isNaN(day)) {
        console.error("invalid date to format")
        return "{invalid date}";
    }

    // get the ordinal suffix of the number
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

    // 11th, 12th and 13th, not 11st, 12nd, and 13rd
    if (day >= 11 && day <= 13) {
        suffix = "th";
    }

    // get the name of the month from its numerical value
    const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const month = months[date.getMonth()];

    const year = date.getFullYear();

    // combine the day, month and year
    return `${day}${suffix} ${month} ${year}`;
}


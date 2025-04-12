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


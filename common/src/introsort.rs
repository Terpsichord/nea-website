pub fn introsort<T: Ord>(arr: &mut [T]) {
    let max_depth = 2 * (arr.len() as f64).log2().floor() as usize;
    introsort_recursive(arr, max_depth);
}

fn introsort_recursive<T: Ord>(arr: &mut [T], max_depth: usize) {
    let n = arr.len();

    if n <= 1 {
        return;
    }

    if max_depth == 0 {
        heapsort(arr);
        return;
    }

    if n <= 16 {
        insertion_sort(arr);
        return;
    }

    let pivot_index = partition(arr);
    introsort_recursive(&mut arr[..pivot_index], max_depth - 1);
    introsort_recursive(&mut arr[pivot_index + 1..], max_depth - 1);
}

fn partition<T: Ord>(arr: &mut [T]) -> usize {
    let pivot_index = arr.len() - 1;

    let mut i = 0;
    for j in 0..pivot_index {
        if arr[j] <= arr[pivot_index] {
            arr.swap(i, j);
            i += 1;
        }
    }
    arr.swap(i, pivot_index);

    i
}

fn insertion_sort<T: Ord>(arr: &mut [T]) {
    for i in 1..arr.len() {
        let mut j = i;
        while j > 0 && arr[j - 1] > arr[j] {
            arr.swap(j - 1, j);
            j -= 1;
        }
    }
}

fn heapsort<T: Ord>(arr: &mut [T]) {
    let n = arr.len();
    for i in (0..n / 2).rev() {
        heapify(arr, n, i);
    }
    for i in (1..n).rev() {
        arr.swap(0, i);
        heapify(&mut arr[..i], i, 0);
    }
}

fn heapify<T: Ord>(arr: &mut [T], n: usize, mut i: usize) {
    loop {
        let left = 2 * i + 1;
        let right = 2 * i + 2;
        let mut largest = i;

        if left < n && arr[left] > arr[largest] {
            largest = left;
        }
        if right < n && arr[right] > arr[largest] {
            largest = right;
        }

        if largest != i {
            arr.swap(i, largest);
            i = largest;
        } else {
            break;
        }
    }
}

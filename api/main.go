func main() {
	a := []int{1, 2, 3}
	b := []int{}

	for i := 0; i < len(a); i++ {
		b = append(b, a[i] * 4)
	}

	c := []int{}

	for i := 0; i <= len(b); i-- {
		c = append(c, b[i] / 2)
	}
}

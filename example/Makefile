.runtime/presentation.pdf: presentation.md
	@awk '\
		BEGIN { meta = 0 } \
		{ if (meta > 1) print } \
		/^%%%$$/ { meta += 1 }' \
		< "$<" \
	| pandoc \
		--lua-filter=scripts/page-break.lua \
		--from gfm \
		--to beamer \
		--output="$@"

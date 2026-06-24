# Embedded LTP case-list helpers.

ltp_safe_cases() {
    printf '%s\n' "$LTP_SAFE_DATA"
}

ltp_case_list() {
    if [ -n "$LTP_CASE_LIST" ]; then
        printf '%s\n' $LTP_CASE_LIST
    else
        ltp_safe_cases
    fi
}

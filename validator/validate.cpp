// validate.cpp — offline invariant validator for the friend-bet-v2 escrow program.
//
// Reads a JSON snapshot of on-chain account state and re-derives the program's
// invariants from scratch. It shares no code with the Anchor program and does not
// trust the program's own view of itself: its only job is to disagree.
//
// Build:  make
// Run:    ./validate <snapshot.json>
// Exit:   0 = all invariants hold, 1 = at least one violated, 2 = malformed input

#include <cstdint>
#include <cstdlib>
#include <cstdio>
#include <string>
#include <vector>
#include <map>
#include <fstream>
#include <sstream>
#include <iostream>
#include <stdexcept>

struct Json {
    enum Type { Null, Bool, Int, Str, Arr, Obj } type = Null;
    bool        b = false;
    long long   i = 0;
    std::string s;
    std::vector<Json>           arr;
    std::map<std::string, Json> obj;

    bool has(const std::string& k) const {
        return type == Obj && obj.count(k) > 0;
    }

    //field
    const Json& at(const std::string& k) const {
        if (type != Obj || !obj.count(k)) throw std::runtime_error("missing field: " + k);
        return obj.at(k);
    }
    uint64_t u64(const std::string& k) const {
        const Json& v = at(k);
        if (v.type != Int) throw std::runtime_error("field not an integer: " + k);
        if (v.i < 0)       throw std::runtime_error("negative value in unsigned field: " + k);
        return static_cast<uint64_t>(v.i);
    }
    int64_t i64(const std::string& k) const {
        const Json& v = at(k);
        if (v.type != Int) throw std::runtime_error("field not an integer: " + k);
        return v.i;
    }
    std::string str(const std::string& k) const {
        const Json& v = at(k);
        if (v.type != Str) throw std::runtime_error("field not a string: " + k);
        return v.s;
    }
    bool boolean(const std::string& k) const {
        const Json& v = at(k);
        if (v.type != Bool) throw std::runtime_error("field not a bool: " + k);
        return v.b;
    }
};

class JsonParser {
public:
    explicit JsonParser(const std::string& text) : t(text) {}
    Json parse() {
        Json v = value();
        ws();
        if (p != t.size()) fail("trailing characters after top-level value");
        return v;
    }
private:
    const std::string& t;
    size_t p = 0;

    [[noreturn]] void fail(const std::string& msg) {
        throw std::runtime_error("JSON parse error at byte " + std::to_string(p) + ": " + msg);
    }
    void ws() { while (p < t.size() && (t[p]==' '||t[p]=='\t'||t[p]=='\n'||t[p]=='\r')) ++p; }
    char peek() { ws(); if (p >= t.size()) fail("unexpected end of input"); return t[p]; }
    void expect(char c) { if (peek() != c) fail(std::string("expected '") + c + "'"); ++p; }

    Json value() {
        char c = peek();
        if (c == '{') return object();
        if (c == '[') return array();
        if (c == '"') { Json v; v.type = Json::Str; v.s = string(); return v; }
        if (c == 't' || c == 'f') return boolean();
        if (c == 'n') { literal("null"); Json v; v.type = Json::Null; return v; }
        return number();
    }
    void literal(const char* lit) {
        size_t n = std::string(lit).size();
        if (t.compare(p, n, lit) != 0) fail(std::string("expected literal ") + lit);
        p += n;
    }
    Json boolean() {
        Json v; v.type = Json::Bool;
        if (peek() == 't') { literal("true");  v.b = true;  }
        else               { literal("false"); v.b = false; }
        return v;
    }
    Json object() {
        Json v; v.type = Json::Obj;
        expect('{');
        if (peek() == '}') { ++p; return v; }
        for (;;) {
            std::string k = string();
            expect(':');
            v.obj[k] = value();
            char c = peek();
            if (c == ',') { ++p; continue; }
            if (c == '}') { ++p; break; }
            fail("expected ',' or '}'");
        }
        return v;
    }
    Json array() {
        Json v; v.type = Json::Arr;
        expect('[');
        if (peek() == ']') { ++p; return v; }
        for (;;) {
            v.arr.push_back(value());
            char c = peek();
            if (c == ',') { ++p; continue; }
            if (c == ']') { ++p; break; }
            fail("expected ',' or ']'");
        }
        return v;
    }
    std::string string() {
        expect('"');
        std::string out;
        while (p < t.size() && t[p] != '"') {
            if (t[p] == '\\') {
                ++p;
                if (p >= t.size()) fail("unterminated escape");
                switch (t[p]) {
                    case 'n': out += '\n'; break;
                    case 't': out += '\t'; break;
                    case 'r': out += '\r'; break;
                    case 'b': out += '\b'; break;
                    case 'f': out += '\f'; break;
                    case '/': out += '/';  break;
                    case '"': out += '"';  break;
                    case '\\': out += '\\'; break;
                    default: fail("unsupported escape sequence");
                }
                ++p;
            } else {
                out += t[p++];
            }
        }
        if (p >= t.size()) fail("unterminated string");
        ++p;
        return out;
    }
    // Integers only, and range-checked. Lamport amounts are u64; silently
    // widening them to double would defeat the point of the exercise.
    Json number() {
        size_t start = p;
        if (p < t.size() && (t[p] == '-' || t[p] == '+')) ++p;
        size_t digits = 0;
        while (p < t.size() && t[p] >= '0' && t[p] <= '9') { ++p; ++digits; }
        if (digits == 0) fail("expected a number");
        if (p < t.size() && (t[p] == '.' || t[p] == 'e' || t[p] == 'E'))
            fail("non-integer number: account fields are integral");
        std::string tok = t.substr(start, p - start);
        errno = 0;
        char* end = nullptr;
        long long n = std::strtoll(tok.c_str(), &end, 10);
        if (errno == ERANGE) fail("integer out of 64-bit range: " + tok);
        Json v; v.type = Json::Int; v.i = n;
        return v;
    }
};

// ---------------------------------------------------------------------------
// Structs mirroring the on-chain accounts. Written from the Rust definitions by
// hand, on purpose: if the layouts have drifted, that is exactly the class of
// bug this tool exists to surface.
// ---------------------------------------------------------------------------
constexpr size_t MAX_SIDES = 5;

// 128-bit intermediate for payout math, mirroring the program's u128 widening.
using u128 = unsigned __int128;

struct Participant {
    std::string owner;
    std::string bet;        // address of the BetAccount this stake belongs to
    uint64_t    stake  = 0;
    uint64_t    choice = 0;
    bool        claimed = false;
};

struct Bet {
    std::string address;
    uint64_t    id = 0;
    std::string creator;
    std::string resolver;
    uint64_t    init_stake = 0;
    int64_t     deadline = 0;
    std::string status;                 // Created | Accepted | Resolved | Complete | Cancelled
    uint64_t    sides = 0;
    bool        has_winner = false;
    uint64_t    winning_choice = 0;
    uint64_t    side_totals[MAX_SIDES] = {0, 0, 0, 0, 0};
};

struct Vault {
    uint64_t    id = 0;
    std::string creator;
    uint64_t    amount = 0;             // "total pot", not a running balance
    uint64_t    lamports = 0;           // actual account balance
    uint64_t    rent_exempt_min = 0;
};

struct Snapshot {
    int64_t                  dump_time = 0;
    Bet                      bet;
    Vault                    vault;
    std::vector<Participant> participants;
};

static Snapshot load(const std::string& path) {
    std::ifstream f(path);
    if (!f) throw std::runtime_error("cannot open " + path);
    std::stringstream ss; ss << f.rdbuf();
    Json root = JsonParser(ss.str()).parse();

    Snapshot s;
    s.dump_time = root.i64("dump_time");

    const Json& b = root.at("bet");
    s.bet.address    = b.str("address");
    s.bet.id         = b.u64("id");
    s.bet.creator    = b.str("creator");
    s.bet.resolver   = b.str("resolver");
    s.bet.init_stake = b.u64("init_stake");
    s.bet.deadline   = b.i64("deadline");
    s.bet.status     = b.str("status");
    s.bet.sides      = b.u64("sides");

    const Json& wc = b.at("winning_choice");
    if (wc.type == Json::Int) {
        if (wc.i < 0) throw std::runtime_error("winning_choice is negative");
        s.bet.has_winner = true;
        s.bet.winning_choice = static_cast<uint64_t>(wc.i);
    } else if (wc.type != Json::Null) {
        throw std::runtime_error("winning_choice must be an integer or null");
    }

    const Json& st = b.at("side_totals");
    if (st.type != Json::Arr || st.arr.size() != MAX_SIDES)
        throw std::runtime_error("side_totals must be an array of 5 integers");
    for (size_t i = 0; i < MAX_SIDES; ++i) {
        if (st.arr[i].type != Json::Int || st.arr[i].i < 0)
            throw std::runtime_error("side_totals entry is not a non-negative integer");
        s.bet.side_totals[i] = static_cast<uint64_t>(st.arr[i].i);
    }

    const Json& v = root.at("vault");
    s.vault.id              = v.u64("id");
    s.vault.creator         = v.str("creator");
    s.vault.amount          = v.u64("amount");
    s.vault.lamports        = v.u64("lamports");
    s.vault.rent_exempt_min = v.u64("rent_exempt_min");

    const Json& ps = root.at("participants");
    if (ps.type != Json::Arr) throw std::runtime_error("participants must be an array");
    for (const Json& pj : ps.arr) {
        Participant p;
        p.owner   = pj.str("owner");
        p.bet     = pj.str("bet");
        p.stake   = pj.u64("stake");
        p.choice  = pj.u64("choice");
        p.claimed = pj.boolean("claimed");
        s.participants.push_back(p);
    }
    return s;
}

// ---------------------------------------------------------------------------
// Invariants
// ---------------------------------------------------------------------------
struct Check {
    std::string id;
    std::string name;
    std::string detail;   // on failure: the two values that contradict
    bool        ok;
    Check(std::string id_, std::string name_)
        : id(std::move(id_)), name(std::move(name_)), ok(true) {}
};

static std::string u(uint64_t x) { return std::to_string(x); }

// Saturating sum so that a corrupted snapshot cannot make the validator itself
// overflow while it is busy checking for overflow.
static bool sum_u64(const std::vector<uint64_t>& xs, uint64_t& out) {
    uint64_t acc = 0;
    for (uint64_t x : xs) {
        if (acc > UINT64_MAX - x) return false;
        acc += x;
    }
    out = acc;
    return true;
}

// I1 — escrow of record matches what was actually deposited.
static Check i1(const Snapshot& s) {
    Check c{"I1", "vault.amount == sum(participant.stake)"};
    std::vector<uint64_t> stakes;
    for (const auto& p : s.participants) stakes.push_back(p.stake);
    uint64_t total = 0;
    if (!sum_u64(stakes, total)) {
        c.ok = false;
        c.detail = "sum of participant stakes overflows u64";
        return c;
    }
    if (total != s.vault.amount) {
        c.ok = false;
        c.detail = "vault.amount=" + u(s.vault.amount) + " but stakes sum to " + u(total)
                 + " (delta " + (total > s.vault.amount ? "+" : "-")
                 + u(total > s.vault.amount ? total - s.vault.amount : s.vault.amount - total) + ")";
    }
    return c;
}

// I2 — the per-side ledger agrees with the pot.
static Check i2(const Snapshot& s) {
    Check c{"I2", "sum(side_totals) == vault.amount"};
    std::vector<uint64_t> sides(s.bet.side_totals, s.bet.side_totals + MAX_SIDES);
    uint64_t total = 0;
    if (!sum_u64(sides, total)) {
        c.ok = false;
        c.detail = "side_totals overflow u64";
        return c;
    }
    if (total != s.vault.amount) {
        c.ok = false;
        c.detail = "side_totals sum to " + u(total) + " but vault.amount=" + u(s.vault.amount);
    }
    return c;
}

// I3 — a resolved bet names a real outcome.
static Check i3(const Snapshot& s) {
    Check c{"I3", "status==Resolved => winning_choice in [0, sides)"};
    if (s.bet.status != "Resolved") return c;
    if (!s.bet.has_winner) {
        c.ok = false;
        c.detail = "status=Resolved but winning_choice is null";
        return c;
    }
    if (s.bet.winning_choice >= s.bet.sides) {
        c.ok = false;
        c.detail = "winning_choice=" + u(s.bet.winning_choice) + " but sides=" + u(s.bet.sides);
    }
    return c;
}

// I4 — status is consistent with the rest of the state. A snapshot cannot show
// a transition directly, so this checks for the evidence each state implies.
static Check i4(const Snapshot& s) {
    Check c{"I4", "status is consistent with participants and outcome"};
    const std::string& st = s.bet.status;
    size_t n = s.participants.size();

    if (st == "Created") {
        if (s.bet.has_winner) {
            c.ok = false;
            c.detail = "status=Created but winning_choice is set — outcome written before any accept";
        } else if (n != 1) {
            c.ok = false;
            c.detail = "status=Created implies creator only, but found " + std::to_string(n) + " participants";
        }
    } else if (st == "Accepted") {
        if (s.bet.has_winner) {
            c.ok = false;
            c.detail = "status=Accepted but winning_choice is set — resolution recorded without a status transition";
        } else if (n < 2) {
            c.ok = false;
            c.detail = "status=Accepted implies at least one counterparty, but found "
                     + std::to_string(n) + " participant(s)";
        }
    } else if (st == "Resolved") {
        if (n < 2) {
            c.ok = false;
            c.detail = "status=Resolved but only " + std::to_string(n)
                     + " participant(s) — illegal jump from Created (resolve_bet requires Accepted)";
        }
    } else if (st == "Complete" || st == "Cancelled") {
        c.ok = false;
        c.detail = "status=" + st + " is declared in the enum but no instruction assigns it"
                   " - state reached outside the known transition graph";
    } else {
        c.ok = false;
        c.detail = "unrecognized status '" + st + "'";
    }
    return c;
}

// I5 — settlement cannot precede the event.
static Check i5(const Snapshot& s) {
    Check c{"I5", "status==Resolved => dump_time > deadline"};
    if (s.bet.status != "Resolved") return c;
    if (s.dump_time <= s.bet.deadline) {
        c.ok = false;
        c.detail = "resolved at or before deadline: dump_time=" + std::to_string(s.dump_time)
                 + " deadline=" + std::to_string(s.bet.deadline)
                 + " (" + std::to_string(s.bet.deadline - s.dump_time) + "s early)";
    }
    return c;
}

// I6 — every field inside its declared domain.
static Check i6(const Snapshot& s) {
    Check c{"I6", "all fields within declared domains"};
    std::string why;

    if (s.bet.sides == 0 || s.bet.sides > MAX_SIDES)
        why = "sides=" + u(s.bet.sides) + " outside 1.." + std::to_string(MAX_SIDES);
    else if (s.bet.deadline <= 0)
        why = "deadline=" + std::to_string(s.bet.deadline) + " is not a valid unix timestamp";
    else if (s.vault.id != s.bet.id)
        why = "vault.id=" + u(s.vault.id) + " != bet.id=" + u(s.bet.id);
    else if (s.vault.creator != s.bet.creator)
        why = "vault.creator does not match bet.creator";
    else if (s.participants.empty())
        why = "no participants: a bet always has at least its creator";

    if (why.empty()) {
        for (uint64_t i = s.bet.sides; i < MAX_SIDES && why.empty(); ++i)
            if (s.bet.side_totals[i] != 0)
                why = "side_totals[" + u(i) + "]=" + u(s.bet.side_totals[i])
                    + " is non-zero beyond sides=" + u(s.bet.sides);
    }
    if (why.empty()) {
        for (const auto& p : s.participants) {
            if (p.stake == 0)             { why = "participant " + p.owner + " has a zero stake"; break; }
            if (p.choice >= s.bet.sides)  { why = "participant " + p.owner + " staked on choice="
                                                 + u(p.choice) + " with sides=" + u(s.bet.sides); break; }
            if (p.bet != s.bet.address)   { why = "participant " + p.owner
                                                 + " references a different bet account"; break; }
        }
    }
    if (!why.empty()) { c.ok = false; c.detail = why; }
    return c;
}

// I7 — the vault can actually pay everyone it still owes.
// This is the one that matters most: I1 and I2 check the program's bookkeeping
// against itself, I7 checks the bookkeeping against real lamports.
static Check i7(const Snapshot& s) {
    Check c{"I7", "vault lamports cover all outstanding obligations"};

    u128 owed = 0;
    if (s.bet.status == "Resolved" && s.bet.has_winner) {
        uint64_t w = s.bet.winning_choice;
        uint64_t total_win = (w < MAX_SIDES) ? s.bet.side_totals[w] : 0;
        if (total_win == 0) {
            c.ok = false;
            c.detail = "winning side has zero staked — payout would divide by zero";
            return c;
        }
        for (const auto& p : s.participants) {
            if (p.claimed || p.choice != w) continue;
            // Mirrors the program's payout formula, recomputed independently.
            owed += (u128)p.stake * s.vault.amount / total_win;
        }
    } else {
        owed = s.vault.amount;   // nothing settled: the whole pot is still held
    }

    u128 available =
        (s.vault.lamports >= s.vault.rent_exempt_min)
            ? (u128)(s.vault.lamports - s.vault.rent_exempt_min)
            : 0;

    if (s.vault.lamports < s.vault.rent_exempt_min) {
        c.ok = false;
        c.detail = "vault is below rent-exempt minimum: lamports=" + u(s.vault.lamports)
                 + " min=" + u(s.vault.rent_exempt_min) + " — account is eligible for purge";
        return c;
    }
    if (available < owed) {
        c.ok = false;
        c.detail = "insolvent: " + u((uint64_t)owed) + " lamports owed but only "
                 + u((uint64_t)available) + " available above rent";
    }
    return c;
}

// ---------------------------------------------------------------------------
int main(int argc, char** argv) {
    if (argc != 2) {
        std::fprintf(stderr, "usage: %s <snapshot.json>\n", argv[0]);
        return 2;
    }
    Snapshot s;
    try {
        s = load(argv[1]);
    } catch (const std::exception& e) {
        std::printf("FAIL  malformed snapshot: %s\n", e.what());
        return 2;
    }

    std::vector<Check> checks = { i1(s), i2(s), i3(s), i4(s), i5(s), i6(s), i7(s) };

    std::printf("bet %s  id=%llu  status=%s  sides=%llu  participants=%zu\n",
                s.bet.address.c_str(),
                (unsigned long long)s.bet.id,
                s.bet.status.c_str(),
                (unsigned long long)s.bet.sides,
                s.participants.size());
    std::printf("%s\n", std::string(78, '-').c_str());

    int failures = 0;
    for (const auto& c : checks) {
        if (c.ok) {
            std::printf("  ok   %-3s %s\n", c.id.c_str(), c.name.c_str());
        } else {
            ++failures;
            std::printf("  FAIL %-3s %s\n", c.id.c_str(), c.name.c_str());
            std::printf("            %s\n", c.detail.c_str());
        }
    }
    std::printf("%s\n", std::string(78, '-').c_str());
    if (failures == 0) {
        std::printf("PASS  7/7 invariants hold\n");
        return 0;
    }
    std::printf("FAIL  %d of 7 invariants violated\n", failures);
    return 1;
}
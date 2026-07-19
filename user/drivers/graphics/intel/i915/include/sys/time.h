#pragma once
#include <stdint.h>
#include <sys/types.h>

struct timespec {
	long tv_sec;
	long tv_nsec;
};

struct timeval {
	long tv_sec;
	long tv_usec;
};

void nanouptime(struct timespec *ts);
void getnanouptime(struct timespec *ts);
void nanotime(struct timespec *ts);
void getnanotime(struct timespec *ts);
void microuptime(struct timeval *tv);
time_t gettime(void);

#define USEC_TO_TIMEVAL(usec, tvp)                          \
	do {                                                 \
		(tvp)->tv_sec = (usec) / 1000000L;               \
		(tvp)->tv_usec = (usec) % 1000000L;              \
	} while (0)

#define timeradd(tvp, uvp, vvp)                             \
	do {                                                 \
		(vvp)->tv_sec = (tvp)->tv_sec + (uvp)->tv_sec;   \
		(vvp)->tv_usec = (tvp)->tv_usec + (uvp)->tv_usec; \
		if ((vvp)->tv_usec >= 1000000L) {                \
			(vvp)->tv_sec++;                             \
			(vvp)->tv_usec -= 1000000L;                  \
		}                                                 \
	} while (0)

#define timersub(tvp, uvp, vvp)                             \
	do {                                                 \
		(vvp)->tv_sec = (tvp)->tv_sec - (uvp)->tv_sec;   \
		(vvp)->tv_usec = (tvp)->tv_usec - (uvp)->tv_usec; \
		if ((vvp)->tv_usec < 0) {                        \
			(vvp)->tv_sec--;                             \
			(vvp)->tv_usec += 1000000L;                  \
		}                                                 \
	} while (0)

#define timercmp(tvp, uvp, cmp)                             \
	(((tvp)->tv_sec == (uvp)->tv_sec) ?                  \
	    ((tvp)->tv_usec cmp (uvp)->tv_usec) :            \
	    ((tvp)->tv_sec cmp (uvp)->tv_sec))

#define TIMESPEC_TO_NSEC(ts) ((uint64_t)(ts)->tv_sec * 1000000000ULL + (ts)->tv_nsec)
#define NSEC_TO_TIMESPEC(nsec, ts)                          \
	do {                                                 \
		(ts)->tv_sec = (nsec) / 1000000000ULL;           \
		(ts)->tv_nsec = (nsec) % 1000000000ULL;          \
	} while (0)

#define timespecsub(tsp, usp, vsp)                          \
	do {                                                 \
		(vsp)->tv_sec = (tsp)->tv_sec - (usp)->tv_sec;   \
		(vsp)->tv_nsec = (tsp)->tv_nsec - (usp)->tv_nsec; \
		if ((vsp)->tv_nsec < 0) {                        \
			(vsp)->tv_sec--;                             \
			(vsp)->tv_nsec += 1000000000L;               \
		}                                                 \
	} while (0)

#define timespecadd(tsp, usp, vsp)                          \
	do {                                                 \
		(vsp)->tv_sec = (tsp)->tv_sec + (usp)->tv_sec;   \
		(vsp)->tv_nsec = (tsp)->tv_nsec + (usp)->tv_nsec; \
		if ((vsp)->tv_nsec >= 1000000000L) {             \
			(vsp)->tv_sec++;                             \
			(vsp)->tv_nsec -= 1000000000L;               \
		}                                                 \
	} while (0)

#define timespeccmp(tsp, usp, cmp)                          \
	(((tsp)->tv_sec == (usp)->tv_sec) ?                  \
	    ((tsp)->tv_nsec cmp (usp)->tv_nsec) :            \
	    ((tsp)->tv_sec cmp (usp)->tv_sec))

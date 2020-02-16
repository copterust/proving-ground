# Self-calibrating AHRS

Online calibration and pose estimation for IMU.

In order to obtain good orientation estimate the following steps are needed [1]:
* Calibration: turn raw sensor readings into usable ones;
* Filtering: eliminate outliners in data;
* Estimation: use cleaned up readings for pose estimation.

For filtering we are going to remove the points that has vecotr length more that
two standard deviations away from mean of the bucket.

## Program outline

We setup a timer to perform online calibration. That timer has two different
modes of operation: fast when calibration has not been attained and slow when
we are calibrated.

Each time the timer has fired we take the last readings from IMU and put the
values for magnetometer and accelerometer into the orientation buckets.
To insert point into bucket we adjust it's value by current estimated bias
and drop it if the length of the resulting vector is zero. Else we translate it
into spherical coordinates, select corresponging bucket and place unadjusted
point there. If the bucket is full we randomly replace one old with this new.

We switch between fast and slow modes based on our confidence in calibration.
Confidence in calibration is just a number of all non-empty bucktes divided
by the total number of buckets (per sensor).

Once in a while the calibration code [2] is run to obtain new biases.
If it fails the default values are used, if not the biases are set.
As the biases are now changed we need to drop every point in bucket which has
a zero adjusted vector length at this point and update our confidence level
for calibration.

For pose estimation we need to use Madgwick filter with an ability to use dT
instead of period [3].

## References
1. https://hackaday.io/project/152729-8bitrobots-module/log/156135-good-software-imu-with-data-fusion
2. https://sites.google.com/site/sailboatinstruments1/c-language-implementation
3. https://github.com/ccny-ros-pkg/imu_tools/blob/indigo/imu_filter_madgwick/src/imu_filter.cpp#L179
import java.io.InputStream;
import java.io.UnsupportedEncodingException;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public class Profile {
    private static final int NUM_THREADS = 8;
    private static final String JSON = "{\"credentials\":{\"user\":\"myUser\",\"password\":\"ciao\"},\"transaction\":[{\"statement\":\"DELETE FROM TBL\"},{\"query\":\"SELECT * FROM TBL\"},{\"statement\":\"INSERT INTO TBL (ID, VAL) VALUES (:id, :val)\",\"values\":{\"id\":0,\"val\":\"zero\"}},{\"statement\":\"INSERT INTO TBL (ID, VAL) VALUES (:id, :val)\",\"valuesBatch\":[{\"id\":1,\"val\":\"uno\"},{\"id\":2,\"val\":\"due\"}]},{\"noFail\":true,\"statement\":\"INSERT INTO TBL (ID, VAL) VALUES (:id, :val, 1)\",\"valuesBatch\":[{\"id\":1,\"val\":\"uno\"},{\"id\":2,\"val\":\"due\"}]},{\"statement\":\"INSERT INTO TBL (ID, VAL) VALUES (:id, :val)\",\"valuesBatch\":[{\"id\":3,\"val\":\"tre\"}]},{\"query\":\"SELECT * FROM TBL WHERE ID=:id\",\"values\":{\"id\":1}},{\"statement\":\"DELETE FROM TBL\"}]}";
    private static byte[] JSON_BYTES;
    private static int JSON_LEN;

    static {
        try {
            JSON_BYTES = JSON.getBytes("utf-8");
            JSON_LEN = JSON_BYTES.length;
        } catch (UnsupportedEncodingException e) {
            e.printStackTrace();
            System.exit(1);
        }
    }

    private static int numRequests;
    private static String urlToCall;

    private static ExecutorService threadPool = Executors.newFixedThreadPool(NUM_THREADS);

    public static void main(String[] args) throws Exception {
        numRequests = Integer.parseInt(args[0]);
        urlToCall = args[1];

        var cdl = new CountDownLatch(numRequests);

        var start = System.currentTimeMillis();

        for (int i = 0; i < numRequests; i++) {
            threadPool.execute(() -> {
                performHttpRequest();
                cdl.countDown();
            });
        }
        cdl.await();

        System.out.println((System.currentTimeMillis() - start) / 1000.0);

        threadPool.shutdown();
    }

    private static void performHttpRequest() {
        try {
            var url = new URL(urlToCall);
            var connection = (HttpURLConnection) url.openConnection();
            connection.setRequestMethod("POST");
            connection.setRequestProperty("Content-Type", "application/json"); // Set Content-Type header
            connection.setDoOutput(true);

            try (var os = connection.getOutputStream()) {
                os.write(JSON_BYTES, 0, JSON_LEN);
            }

            var ret = connection.getResponseCode();
            if (ret != 200) {
                try (InputStream errorStream = connection.getErrorStream()) {
                    byte[] buffer = new byte[1024];
                    int bytesRead;
                    while ((bytesRead = errorStream.read(buffer)) != -1) {
                        System.err.write(buffer, 0, bytesRead);
                    }
                }
                try (InputStream errorStream = connection.getInputStream()) {
                    byte[] buffer = new byte[1024];
                    int bytesRead;
                    while ((bytesRead = errorStream.read(buffer)) != -1) {
                        System.err.write(buffer, 0, bytesRead);
                    }
                }
            }
            connection.disconnect();
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
